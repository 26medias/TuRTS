// turts-test-role: unit
'use strict';
const assert = require('node:assert/strict');
const test = require('node:test');
const vm = require('node:vm');
const {installDeviceAudit, collectDeviceAudit} = require('./device-audit.js');

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
    assert.equal(r.audit.status, 'installed');
});

test('does not inspect descriptors, returned then or error getters; rethrows same error', () => {
    const descriptor = new Proxy({}, {get(){throw Error('descriptor read');}});
    const result = new Proxy({}, {get(){throw Error('returned property read');}});
    const r = realm(() => result);
    assert.equal(r.adapter.requestDevice(descriptor), result);
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
