#!/usr/bin/env python3
"""Prepare an editable-terrain variant of the accepted SDK16 battle400 fixture. Never builds/runs."""
import argparse,hashlib,json,os,re,shutil
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('accepted',type=Path);p.add_argument('output',type=Path);a=p.parse_args()
src=a.accepted.resolve();out=a.output.resolve();source=src/'battle400';text=(source/'maps/field.ron').read_text()
placements=list(re.finditer(r'unit:\s*"skirmish:(warrior|swordsman|soldier)",\s*owner_slot:\s*([01]),\s*pos:\s*\(x:\s*([0-9.]+),\s*y:\s*([0-9.]+)\)',text))
if len(placements)!=400:raise ValueError('Requires the unchanged accepted three-type400 battle')
if re.search(r'\bterrain\s*:',text):raise ValueError('Source already has terrain')
size=re.search(r'id:\s*"field",\s*size:\s*\(x:\s*([0-9.]+),\s*y:\s*([0-9.]+)\)',text);width,height=map(float,size.groups())
if width<10 or height<32:raise ValueError('Accepted source needs the existing clear lower landscape margin')
columns,rows=round(width*5),round(height*5)
if abs(columns/5-width)>1e-5 or abs(rows/5-height)>1e-5:raise ValueError('Map extent must have an exact .2-world-unit cell grid')
candidates=[(i+1,float(m[3]),float(m[4])) for i,m in enumerate(placements) if m[1]=='soldier' and m[2]=='0']
actor,x,y=min(candidates,key=lambda row:(-row[2],row[1]))
work_x=int(x*5);work_y=int((y+2.1)*5);ref_y=int((y+4.8)*5)
if not(120<=ref_y<160):raise ValueError('Expected accepted front combat row near y20.2')
out.mkdir(parents=True,exist_ok=False)
def copy(src,dst):
 if str(src).endswith('.ron'):return shutil.copy2(src,dst)
 try:os.link(src,dst);return dst
 except OSError:return shutil.copy2(src,dst)
package=out/'battle400';shutil.copytree(source,package,copy_function=copy);shutil.copy2(src/'battle400-commands.json',out/'battle400-commands.json')
def put(file,value):
 target=package/file;target.parent.mkdir(parents=True,exist_ok=True);target.write_text(value+'\n')
manifest=package/'manifest.ron';m=re.sub(r'engine_api:\s*"[^"]+"','engine_api:"^0.2.17"',manifest.read_text());m=m.replace('exports:','exports:',1)
# Add records to existing export lists; preserve every accepted asset/content export.
for key,path in [('terrain_materials','terrain/material.ron'),('terrain_movement_classes','terrain/class.ron'),('terrain_tools','terrain/tool.ron')]:
 if re.search(r'\b'+key+r'\s*:',m):raise ValueError('Unexpected terrain export in baseline')
 m=re.sub(r'(exports:\s*\()',r'\1'+key+':["'+path+'"],',m,count=1)
m=re.sub(r'(orders:\s*\[)',r'\1"orders/terrain_work.ron",',m,count=1);manifest.write_text(m)
put('terrain/material.ron','(id:"benchmark_soil",name:"Benchmark soil",surface_color:"#856b48",hardness:1,work_per_world_cube:(numerator:4000,denominator:1),natural_outputs:[(resource:"skirmish:credits",per_world_cube:(numerator:200,denominator:1))],fill_inputs:[(resource:"skirmish:credits",per_world_cube:(numerator:200,denominator:1))],placed_removal:NoYield)')
put('terrain/class.ron','(id:"benchmark_ground",name:"Benchmark ground",max_step:0,max_grade:(numerator:0,denominator:1))')
put('terrain/tool.ron','(id:"benchmark_cut",name:"Benchmark cut",operation:Excavate,work_per_second:10,reach:16.0,allowed_materials:["skirmish:benchmark_soil"],max_hardness:1,output_destination:SlotBalances)')
put('orders/terrain_work.ron','(id:"benchmark_work",name:"Benchmark work",behaviour:TerrainWork,target:TerrainDesignation,fire:None,engage:None,icon:"icon_order_move",hotkey:"key_benchmark_work",cursor:None,card:None,units:["skirmish:soldier"],description:"Work the public exposed cell")')
for kind in ['soldier','swordsman','warrior']:
 f=package/'units'/(kind+'.ron');s=f.read_text();s=s.replace('(', '(terrain_class:Some("skirmish:benchmark_ground"),'+('terrain_tools:["skirmish:benchmark_cut"],' if kind=='soldier' else ''),1)
 if 'vision:' not in s:s=s.replace('(', '(vision:Some((radius:8.0)),',1)
 f.write_text(s)
# Catalog statics keep their original fields and explicitly protect their support.
for f in (package/'units').glob('*.ron'):
 s=f.read_text()
 if not re.search(r'\bmovement:\s*Some',s) and 'terrain_foundation:' not in s:
  f.write_text(s.replace('(', '(terrain_foundation:Some(ForbidEdit),',1))
terrain=f'terrain:Some((length_scale:1000,cell_width:200,elevation_step:125,columns:{columns},rows:{rows},chunk_width:16,initial_elevation:0,initial_heights:[(cells:(min:(x:30,y:120),max:(x:50,y:160)),elevation:-32)],floor:-40,ceiling:4,generation:Version1(bands:[(bottom:-40,top:0,choices:[(material:"skirmish:benchmark_soil",weight:1)])]))),'
text=text.replace('(', '('+terrain,1);put('maps/field.ron',text)
f=package/'input/bindings.ron';f.write_text(re.sub(r'(actions:\s*\[)',r'\1(name:"key_benchmark_work",default:[]),',f.read_text(),count=1))
f=package/'interface/hud.ron';s=f.read_text();widget='(id:"terrain_benchmark_tools",at:Some((anchor:TopLeft,offset:(12.0,70.0),pivot:None)),size:(w:Px(210.0),h:Px(30.0)),visible:[PrimaryOwned],widget:List(source:TerrainTools(order:"skirmish:benchmark_work"),direction:Vertical,gap:2,max_items:1,item:(id:"benchmark_tool",size:(w:Fill,h:Px(28.0)),widget:Button(icon:None,label:Some(Item(Name)),action:Item(Action),hotkey_label:false)))), '
s=re.sub(r'(root:\s*\[)',r'\1'+widget,s,count=1);f.write_text(s)
bounds={key:float(re.search(key+r':\s*([0-9.]+)',s)[1]) for key in ['minimum','maximum']}
camera=re.search(r'camera:\s*\((.*?)\),',s,re.S)[1]
bounds['normal']=float(re.search(r'\bheight:\s*([0-9.]+)',camera)[1])
meta={'status':'PREPARED_NOT_RUN','source':str(source),'source_map_sha256':hashlib.sha256((source/'maps/field.ron').read_bytes()).hexdigest(),'commands_sha256':hashlib.sha256((src/'battle400-commands.json').read_bytes()).hexdigest(),'actors':400,'unchanged':'all400 actor placements, model assets, animation clips, health, weapons, camera bounds, accepted command-script bytes','added':'public editable terrain and one existing soldier assigned work through real UI; combat activity is reported separately','grid':{'columns':columns,'rows':rows,'cell_width':.2,'step':.125},'sample_ms':10000,'warmup_ms':2000,'fps_floor':30,'quality':'high','zooms':['normal','near','far'],'work':{'actor':actor,'tool':'skirmish:benchmark_cut','reference':[(work_x+.5)/5,(ref_y+.5)/5,-4],'cell':{'x':work_x,'y':work_y},'point':[(work_x+.5)/5,(work_y+.5)/5,0],'target_elevation':-32,'required_work_per_prism':20,'work_per_second':10,'expected_full_duration_seconds':64},'camera_bounds':bounds}
meta['public_files_sha256']={str(p.relative_to(package)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(package.rglob('*')) if p.is_file()}
(out/'terrain-fps.json').write_text(json.dumps(meta,indent=2)+'\n');print(json.dumps(meta,indent=2))
