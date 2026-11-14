// Execute production skin math on the real GPU. Only mesh input bindings are replaced by fixtures.
const fs=require('node:fs'),http=require('node:http'),crypto=require('node:crypto');
const {chromium}=require('playwright-core');
const [shaderPath,fixturePath,out]=process.argv.slice(2);if(!out)throw Error('SHADER FIXTURE OUTPUT');
fs.mkdirSync(out,{recursive:true});const fixture=JSON.parse(fs.readFileSync(fixturePath));
if(!fixture.cases?.length||!fixture.matrices?.length)throw Error('Empty native reference fixture');
const source=fs.readFileSync(shaderPath,'utf8'),importLine='import bevy_pbr::render::mesh_functions;';
if(source.split(importLine).length!==2)throw Error('Unexpected production shader imports');
const production=source.replace(importLine,'')
 .replaceAll('mesh_functions::get_tag','reference_get_tag')
 .replaceAll('mesh_functions::get_world_from_local','reference_get_world_from_local');
if(production.includes('::'))throw Error('The shader adapter needs an explicit update');
(async()=>{let context,server;const profile=fs.mkdtempSync('/tmp/turts-shared-skin-math-');
const result={status:'RUNNING',source_shader_sha256:crypto.createHash('sha256').update(source).digest('hex'),cases:fixture.cases.length,profile};
try{
 server=http.createServer((req,res)=>{res.writeHead(200,{'Content-Type':'text/html'});res.end('<!doctype html><title>Skin math probe</title>');});
 await new Promise(r=>server.listen(0,'127.0.0.1',r));
 context=await chromium.launchPersistentContext(profile,{executablePath:'/usr/bin/google-chrome',headless:false,viewport:{width:1280,height:720},ignoreDefaultArgs:['--enable-unsafe-swiftshader'],args:['--enable-gpu','--alsa-output-device=null','--enable-unsafe-webgpu','--enable-features=Vulkan','--use-angle=vulkan','--disable-vulkan-surface','--no-first-run','--no-default-browser-check']});
 const page=context.pages()[0];await page.goto('http://127.0.0.1:'+server.address().port);
 result.gpu=await page.evaluate(async({production,fixture})=>{
  const adapter=await navigator.gpu.requestAdapter();if(!adapter||adapter.info.isFallbackAdapter)throw Error('Real GPU required');
  const device=await adapter.requestDevice();device.pushErrorScope('validation');
  const stride=160,inputs=new ArrayBuffer(fixture.cases.length*stride),view=new DataView(inputs);
  fixture.cases.forEach((c,i)=>{
   const base=i*stride;let offset=0;
   const floats=values=>values.forEach(value=>{view.setFloat32(base+offset,value,true);offset+=4});
   const uints=values=>values.forEach(value=>{view.setUint32(base+offset,value,true);offset+=4});
   floats(c.anchor);uints(c.indices);floats(c.weights);floats(c.position);floats(c.normal);floats(c.tangent);uints([c.tag,0,0,0]);
   if(offset!==stride)throw Error('GPU case layout mismatch');
  });
  const create=(bytes,usage)=>{const buffer=device.createBuffer({size:bytes.byteLength,usage});device.queue.writeBuffer(buffer,0,bytes);return buffer};
  const cases=create(inputs,GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST);
  const palettes=create(new Float32Array(fixture.matrices.flat()),GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST);
  const outputBytes=fixture.cases.length*48;
  const output=device.createBuffer({size:outputBytes,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC});
  const readback=device.createBuffer({size:outputBytes,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});
  const code=production+`
struct Case { anchor:mat4x4<f32>, indexes:vec4<u32>, weights:vec4<f32>, position:vec4<f32>, normal:vec4<f32>, tangent:vec4<f32>, tag:vec4<u32> };
struct Result { position:vec4<f32>, normal:vec4<f32>, tangent:vec4<f32> };
@group(0) @binding(0) var<storage, read> cases:array<Case>;
@group(0) @binding(1) var<storage, read_write> results:array<Result>;
fn reference_get_tag(index:u32)->u32 { return cases[index].tag.x; }
fn reference_get_world_from_local(index:u32)->mat4x4<f32> { return cases[index].anchor; }
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 if id.x>=arrayLength(&cases) { return; }
 let c=cases[id.x];
 let world=skin_model(c.indexes,c.weights,id.x);
 results[id.x].position=world*c.position;
 results[id.x].normal=vec4<f32>(skin_normals(world,c.normal.xyz),0.0);
 let linear=mat3x3<f32>(world[0].xyz,world[1].xyz,world[2].xyz);
 let anchor=mat3x3<f32>(c.anchor[0].xyz,c.anchor[1].xyz,c.anchor[2].xyz);
 let sign=select(1.0,-1.0,determinant(anchor)<0.0);
 results[id.x].tangent=vec4<f32>(normalize(linear*c.tangent.xyz),c.tangent.w*sign);
}`;
  const module=device.createShaderModule({code});const info=await module.getCompilationInfo();
  const errors=info.messages.filter(m=>m.type==='error');if(errors.length)throw Error(errors.map(m=>`${m.lineNum}:${m.linePos} ${m.message}`).join('\n'));
  const pipeline=await device.createComputePipelineAsync({layout:'auto',compute:{module,entryPoint:'main'}});
  const groups=[device.createBindGroup({layout:pipeline.getBindGroupLayout(0),entries:[{binding:0,resource:{buffer:cases}},{binding:1,resource:{buffer:output}}]}),
   device.createBindGroup({layout:pipeline.getBindGroupLayout(1),entries:[]}),device.createBindGroup({layout:pipeline.getBindGroupLayout(2),entries:[]}),
   device.createBindGroup({layout:pipeline.getBindGroupLayout(3),entries:[{binding:100,resource:{buffer:palettes}}]})];
  const encoder=device.createCommandEncoder(),pass=encoder.beginComputePass();pass.setPipeline(pipeline);groups.forEach((group,i)=>pass.setBindGroup(i,group));pass.dispatchWorkgroups(Math.ceil(fixture.cases.length/64));pass.end();encoder.copyBufferToBuffer(output,0,readback,0,outputBytes);device.queue.submit([encoder.finish()]);
  await readback.mapAsync(GPUMapMode.READ);const values=Array.from(new Float32Array(readback.getMappedRange().slice(0)));readback.unmap();
  const validation=await device.popErrorScope();if(validation)throw Error(validation.message);
  const gpu={adapter:{vendor:adapter.info.vendor,architecture:adapter.info.architecture,description:adapter.info.description,isFallbackAdapter:adapter.info.isFallbackAdapter},max_position_error:0,max_normal_error:0,max_tangent_error:0,values};
  fixture.cases.forEach((c,i)=>{
   for(const [field,offset,tolerance]of [['position',0,0.0002],['normal',4,0.00002],['tangent',8,0.00002]]){
    c['expected_'+field].forEach((expected,k)=>{const actual=values[i*12+offset+k],error=Math.abs(actual-expected);if(!Number.isFinite(actual)||error>tolerance)throw Error(`GPU skin reference mismatch: case ${i} ${field}[${k}] expected ${expected}, got ${actual}`);gpu['max_'+field+'_error']=Math.max(gpu['max_'+field+'_error'],error)});
   }
  });
  device.destroy();return gpu;
 },{production,fixture});
 result.status='PASS';console.log(JSON.stringify({status:result.status,cases:result.cases,position_error:result.gpu.max_position_error,normal_error:result.gpu.max_normal_error,tangent_error:result.gpu.max_tangent_error}));
}catch(e){result.status='FAIL';result.error=String(e.stack||e);console.error(e);process.exitCode=1;}
finally{if(context)await context.close();if(server)await new Promise(r=>server.close(r));fs.rmSync(profile,{recursive:true,force:true});result.profile_removed=!fs.existsSync(profile);fs.writeFileSync(out+'/result.json',JSON.stringify(result,null,2));}
})();
