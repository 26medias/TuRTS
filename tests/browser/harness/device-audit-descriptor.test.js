// turts-test-role: unit
'use strict';
const assert = require('node:assert/strict');
const test = require('node:test');
const vm = require('node:vm');
const {installDeviceAudit, collectDeviceAudit} = require('./device-audit-descriptor.js');

function realm(original, limit = 2) {
    class GPUAdapter {}
    Object.defineProperty(GPUAdapter.prototype, 'requestDevice', {
        configurable: true, writable: true, value: original,
    });
    let clock = 0;
    const context = vm.createContext({GPUAdapter, performance: {now: () => ++clock}});
    vm.runInContext(`(${installDeviceAudit.toString()})({key:'opaque',limit:${limit}})`, context);
    return {adapter: new GPUAdapter(), audit: context.opaque};
}

test('receiver, all argument identities and returned pending promise remain exact', () => {
    const pending = new Promise(() => {});
    let receiver; let received;
    const r = realm(function (...args) {receiver=this;received=args;return pending;});
    const descriptor = {}; const extra = {};
    assert.equal(r.adapter.requestDevice(descriptor, extra), pending);
    assert.equal(receiver, r.adapter);
    assert.equal(received.length, 2);
    assert.equal(received[0], descriptor); assert.equal(received[1], extra);
    assert.equal(r.audit.calls[0].promise, pending);
    assert.equal(r.audit.calls[0].descriptor, descriptor);
    assert.equal(r.audit.status, 'installed');
});

test('does not inspect descriptors, returned then or error getters; rethrows same error', () => {
    const descriptor = new Proxy({}, {get(){throw Error('descriptor read');}});
    const result = new Proxy({}, {get(){throw Error('returned property read');}});
    const r = realm(() => result);
    assert.equal(r.adapter.requestDevice(descriptor), result);
    assert.equal(r.audit.calls[0].descriptor, descriptor);
    const error = new Proxy({}, {get(){throw Error('error property read');}});
    const s = realm(() => {throw error;});
    assert.throws(() => s.adapter.requestDevice(), value => value === error);
    assert.equal(s.audit.calls[0].synchronousThrow, true);
});

test('records bounded calls and overflow without changing subsequent results', () => {
    const promise = Promise.resolve(1); const r = realm(() => promise, 2);
    for(let n=0;n<5;n++) assert.equal(r.adapter.requestDevice(), promise);
    assert.equal(r.audit.total, 5); assert.equal(r.audit.calls.length, 2); assert.equal(r.audit.overflow, 3);
});

test('CDP reads only promise state, never awaits, and releases handles', async () => {
    const sent=[];
    const fake={send:async(method,args)=>{
        sent.push({method,args});
        if(method==='Runtime.evaluate' && args.returnByValue) return {result:{value:{calls:[{call:1}]}}};
        if(method==='Runtime.evaluate') return {result:{objectId:'actual-promise'}};
        if(method==='Runtime.getProperties') return {internalProperties:[{name:'[[PromiseState]]',value:{value:'pending'}}]};
        return {};
    }};
    const result=await collectDeviceAudit(fake,'opaque','bounded-audit');
    assert.equal(result.calls[0].promiseState,'pending');
    assert.equal(result.calls[0].settlementTime,'NOT TRACED');
    assert(sent.filter(x=>x.method==='Runtime.evaluate').every(x=>x.args.awaitPromise===false));
    assert.equal(sent.at(-1).method,'Runtime.releaseObjectGroup');
});

test('CDP failure records NOT TRACED and still attempts handle release', async () => {
    const methods=[];const fake={send:async(method)=>{methods.push(method);if(method!=='Runtime.releaseObjectGroup')throw Error('CDP unavailable');return {};}};
    const result=await collectDeviceAudit(fake,'opaque','bounded-audit');
    assert.equal(result.status,'NOT TRACED');
    assert.equal(methods.at(-1),'Runtime.releaseObjectGroup');
});

test('diagnostic timer failure does not suppress or replace the original call', () => {
    const promise = new Promise(() => {});let count=0;
    class GPUAdapter { requestDevice(){count++;return promise;} }
    const context=vm.createContext({GPUAdapter,performance:{now(){throw Error('timer unavailable');}}});
    vm.runInContext(`(${installDeviceAudit.toString()})({key:'opaque',limit:2})`,context);
    assert.equal(new GPUAdapter().requestDevice(),promise);
    assert.equal(count,1);assert.equal(context.opaque.recordErrors,2);
});

test('missing API is recorded explicitly', () => {
    const context=vm.createContext({});
    vm.runInContext(`(${installDeviceAudit.toString()})({key:'opaque',limit:2})`,context);
    assert.equal(context.opaque.status,'unavailable');
});

const {readDeviceRequirements}=require('./device-audit-descriptor.js');
function descriptorCdp(fields) {
    const commands=[];
    return {commands,send:async(method,args)=>{
        commands.push({method,args});
        if(method==='Runtime.evaluate')return {result:{type:'object',objectId:'descriptor'}};
        assert.equal(method,'Runtime.getProperties');
        return {result:fields[args.objectId]};
    }};
}
test('C312 reads bounded own data features/limits and ignores unrelated accessor metadata',async()=>{
    const cdp=descriptorCdp({
        descriptor:[{name:'requiredFeatures',value:{subtype:'array',objectId:'features'}},{name:'requiredLimits',value:{objectId:'limits'}},{name:'label',get:{objectId:'never-invoke'}}],
        features:[{name:'0',value:{type:'string',value:'timestamp-query'}},{name:'length',value:{type:'number',value:1}}],
        limits:[{name:'maxBufferSize',value:{type:'number',value:268435456}}],
    });
    const value=await readDeviceRequirements(cdp,'opaque',0,'bounded');
    assert.equal(value.status,'observed');assert.deepEqual(value.requiredFeatures,['timestamp-query']);
    assert.equal(value.requiredLimits.maxBufferSize,268435456);
    assert(cdp.commands.every(c=>['Runtime.evaluate','Runtime.getProperties'].includes(c.method)));
    assert(!cdp.commands.some(c=>c.args.objectId==='never-invoke'));
});
test('C312 required accessor is NOT TRACED and is never invoked',async()=>{
    const cdp=descriptorCdp({descriptor:[{name:'requiredFeatures',get:{objectId:'never-invoke'}}]});
    const value=await readDeviceRequirements(cdp,'opaque',0,'bounded');
    assert.equal(value.status,'NOT TRACED');assert.match(value.reason,/accessor/);
    assert.equal(cdp.commands.length,2);
});
test('C312 refuses feature overflow and unsupported limit data',async()=>{
    const cdp=descriptorCdp({descriptor:[{name:'requiredFeatures',value:{subtype:'array',objectId:'features'}}],features:[{name:'length',value:{type:'number',value:65}}]});
    assert.equal((await readDeviceRequirements(cdp,'opaque',0,'bounded')).status,'NOT TRACED');
    const limit=descriptorCdp({descriptor:[{name:'requiredLimits',value:{objectId:'limits'}}],limits:[{name:'maxBufferSize',get:{objectId:'never-invoke'}}]});
    assert.equal((await readDeviceRequirements(limit,'opaque',0,'bounded')).status,'NOT TRACED');
});
