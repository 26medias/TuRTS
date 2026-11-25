#!/usr/bin/env python3
"""Author public M4b fixtures from frozen physical inputs. No builds, private terrain or simulation."""
import argparse,copy,hashlib,json,math,os,re,shutil,struct
from fractions import Fraction
from pathlib import Path
SCENES=Path(__file__).with_name('scenes.json')


def exact_f32(value):return Fraction.from_float(struct.unpack('<f',struct.pack('<f',value))[0])

def sha(file):return hashlib.sha256(file.read_bytes()).hexdigest()
def field(text,name,value):
    pattern=r'\b'+re.escape(name)+r'\s*:\s*[^,)]+'
    result,n=re.subn(pattern,name+':'+value,text,count=1)
    if n!=1:raise ValueError('missing simple public field '+name)
    return result


def patches(scene):
    terrain=scene['map']
    if 'override_patches' in terrain:return terrain['override_patches']
    low,high=terrain['passage_y']
    return [{'min':[30,0],'max':[34,low],'elevation':4},{'min':[30,high],'max':[34,48],'elevation':4}]


def elevation(scene,x,y):
    for patch in patches(scene):
        if patch['min'][0]<=x<patch['max'][0] and patch['min'][1]<=y<patch['max'][1]:return patch['elevation']
    return 0


def profile(data,scene,actor):
    value=copy.deepcopy(data['profiles'][actor['kind']])
    if 'radius_override' in scene:
        value['visual_scale']*=scene['radius_override']/value['radius'];value['pick_height']*=scene['radius_override']/value['radius'];value['radius']=scene['radius_override']
    if 'speed_override' in scene:value['speed']=scene['speed_override']
    if 'profile_override' in scene:
        value['max_step']=scene['profile_override']['max_step'];value['max_grade']=scene['profile_override']['grade']
    return value


def validate(data,scene):
    actors=scene['actors'];names={a['label'] for a in actors}
    if len(names)!=len(actors):raise ValueError('duplicate physical label')
    if scene['row'] in ['GM1','GM2'] and (sum(a['kind']=='small' for a in actors),sum(a['kind']=='large' for a in actors))!=(24,12):raise ValueError('frozen36-unit cohort changed')
    for i,a in enumerate(actors):
        radius=exact_f32(profile(data,scene,a)['radius']);x,y=map(exact_f32,a['position'])
        if not(radius<=x<=64-radius and radius<=y<=48-radius):raise ValueError('initial footprint outside map '+a['label'])
        # Full authored body must start on one plateau, including cells touched with positive area.
        support=set()
        for cy in range(max(0,math.floor(y-radius)),min(48,math.ceil(y+radius))):
            for cx in range(max(0,math.floor(x-radius)),min(64,math.ceil(x+radius))):
                dx=max(cx-x,0,x-(cx+1));dy=max(cy-y,0,y-(cy+1))
                if dx*dx+dy*dy<radius*radius:support.add(elevation(scene,cx,cy))
        if len(support)!=1:raise ValueError('initial footprint straddles cliff '+a['label'])
        for b in actors[:i]:
            bx,by=map(exact_f32,b['position']);distance_squared=(x-bx)**2+(y-by)**2;required=radius+exact_f32(profile(data,scene,b)['radius'])
            if distance_squared<required*required:raise ValueError('overlapping authored bodies '+a['label']+' / '+b['label'])
    for command in scene['commands']:
        if not set(command['actors'])<=names:raise ValueError('command references an absent physical actor')
        if command['action'] in ['fill','excavate']:
            rect=command['cells'];count=(rect['max'][0]-rect['min'][0])*(rect['max'][1]-rect['min'][1])
            if not 1<=count<=256:raise ValueError('work rectangle outside ordinary API bounds')
    if scene['row']=='GM3' and 'bays' in scene:
        for a in actors:
            if a['label']=='blocker':continue
            radius=profile(data,scene,a)['radius'];y=a['position'][1]
            if abs(y-23.5)<radius+1.1:raise ValueError('waiting bay blocks the declared large transit strip')


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--root',type=Path,default=Path(__file__).resolve().parents[3]);parser.add_argument('--package',choices=['quarry','terrain_stockyard'],required=True);parser.add_argument('--scene',required=True);parser.add_argument('--output',type=Path,required=True);parser.add_argument('--reverse-allocation',action='store_true');parser.add_argument('--reverse-members',action='store_true');args=parser.parse_args()
    data=json.loads(SCENES.read_text());scene=copy.deepcopy(data['scenes'][args.scene]);binding=data['packages'][args.package];validate(data,scene)
    root=args.root.resolve();source=root/'mods'/args.package;out=args.output.resolve();out.mkdir(parents=True,exist_ok=False);package=out/args.package
    def copy_file(src,dst):
        relative=Path(src).relative_to(source)
        immutable=relative.parts[0]=='assets' and Path(src).suffix.lower() in {'.glb','.png','.jpg','.jpeg','.webp','.ogg','.wav','.mp3','.ttf','.woff2'}
        if not immutable:return shutil.copy2(src,dst)
        try:os.link(src,dst);return dst
        except OSError:return shutil.copy2(src,dst)
    shutil.copytree(source,package,copy_function=copy_file)
    def put(file,text):
        path=package/file;path.parent.mkdir(parents=True,exist_ok=True)
        # Never write through a shared inode, even if a future fixture edits an asset.
        path.unlink(missing_ok=True);path.write_text(text+'\n')
    prefix=args.package+':'
    # Modify only public actor definitions. Model files/animation clips stay byte-identical.
    for role in ['small','large']:
        kinds=[a for a in scene['actors'] if a['kind']==role];kind=kinds[0] if kinds else {'kind':role};p=profile(data,scene,kind)
        unit=package/'units'/(binding[role]+'.ron');text=unit.read_text();text=re.sub(r'footprint:\s*Circle\(radius:[^)]+\)',f'footprint:Circle(radius:{p["radius"]:.17g})',text,count=1);text=re.sub(r'movement:\s*Some\(\(speed:[^)]+\)\)',f'movement:Some((speed:{p["speed"]}))',text,count=1);text=re.sub(r'capacity:\s*4\b','capacity:256',text);unit.write_text(text)
        cls=package/'classes'/('light.ron' if role=='small' else 'wide.ron');ct=field(cls.read_text(),'max_step',str(p['max_step']));ct=re.sub(r'max_grade:\s*\(numerator:\s*\d+,\s*denominator:\s*\d+\)',f'max_grade:(numerator:{p["max_grade"][0]},denominator:{p["max_grade"][1]})',ct);cls.write_text(ct)
        visual=package/'visuals'/(binding[role]+'.ron');visual.write_text(field(visual.read_text(),'scale',f'{p["visual_scale"]:.9g}'))
    if any(a['kind']=='blocker' for a in scene['actors']):
        # A public larger instance of the existing detailed vehicle model.
        text=(package/'units'/(binding['large']+'.ron')).read_text();text=text.replace('id:"'+binding['large']+'"','id:"settled_blocker"',1);text=text.replace('name:"'+binding['large'].title()+'"','name:"Settled blocker"',1);text=re.sub(r'footprint:\s*Circle\(radius:[^)]+\)','footprint:Circle(radius:1.4)',text,count=1);put('units/settled_blocker.ron',text)
        visual=(package/'visuals'/(binding['large']+'.ron')).read_text().replace(prefix+binding['large'],prefix+'settled_blocker');visual=field(visual,'scale',str(data['profiles']['blocker']['visual_scale']));put('visuals/settled_blocker.ron',visual)
        manifest=package/'manifest.ron';mt=manifest.read_text();mt=re.sub(r'(units:\s*\[)',r'\1"units/settled_blocker.ron",',mt,count=1);mt=re.sub(r'(visuals:\s*\[)',r'\1"visuals/settled_blocker.ron",',mt,count=1);manifest.write_text(mt)
        for name in ['move','stop','hold']:
            order=package/'orders'/(name+'.ron');order.write_text(re.sub(r'(units:\s*\[)',r'\1"'+prefix+'settled_blocker",',order.read_text(),count=1))
    # Copied constructible definitions must remain valid for the newly authored
    # mobile footprints, even when no building is placed in this movement scene.
    # Match the public validator's f32 diameter-plus-clearance arithmetic exactly.
    f32=lambda value:struct.unpack('<f',struct.pack('<f',value))[0]
    unit_files=sorted((package/'units').glob('*.ron'))
    largest=0.0
    for file in unit_files:
        text=file.read_text()
        if re.search(r'\bmovement:\s*Some\(',text):
            radius=re.search(r'\bfootprint:\s*Circle\(radius:\s*([^)]*)\)',text)
            if radius is None:raise ValueError('missing public mobile circle '+str(file))
            largest=max(largest,f32(f32(float(radius.group(1)))*2.0))
    required_gap=f32(largest+f32(.2))
    for file in unit_files:
        text=file.read_text()
        if re.search(r'\bplacement:\s*Some\(',text):
            gap=re.search(r'\bgap:\s*([^,)]+)',text)
            if gap is None:raise ValueError('missing public placement gap '+str(file))
            if f32(float(gap.group(1)))<required_gap:
                file.write_text(field(text,'gap',f'{required_gap:.17g}'))
    for file in (package/'materials').glob('*.ron'):
        # Both packages now use the frozen unit-width/quarter-step prism. Every rate is exact.
        text=re.sub(r'\bper_world_cube:\s*\(numerator:\s*\d+,\s*denominator:\s*\d+\)','per_world_cube:(numerator:4,denominator:1)',file.read_text());file.write_text(text)
    for tool in (binding['dig'],binding['fill']):
        file=package/'tools'/(tool+'.ron');file.write_text(field(file.read_text(),'work_per_second','64'))
    actors=list(scene['actors']);
    if args.reverse_allocation:actors.reverse()
    mapping={a['label']:i+1 for i,a in enumerate(actors)}
    def content(a):return prefix+('settled_blocker' if a['kind']=='blocker' else binding[a['kind']])
    rows=[]
    for a in actors:
        contents='[]'
        if scene.get('resources',{}).get('editor')==a['label'] and binding['destination']=='Inventory':contents=f'[(item:"{prefix+binding["resource"]}",amount:32)]'
        x,y=a['position'];rows.append(f'(unit:"{content(a)}",owner_slot:{a["slot"]},pos:(x:{x},y:{y}),contents:{contents})')
    accounts='[]'
    if 'resources' in scene and binding['destination']=='SlotBalances':accounts=f'[(slot:0,balances:[(balance:"{prefix+binding["resource"]}",amount:32)])]'
    patch_rows=','.join(f'(cells:(min:(x:{p["min"][0]},y:{p["min"][1]}),max:(x:{p["max"][0]},y:{p["max"][1]})),elevation:{p["elevation"]})' for p in patches(scene))
    highest=max([0]+[p['elevation'] for p in patches(scene)])
    terrain=f'terrain:Some((length_scale:1000,cell_width:1000,elevation_step:250,columns:64,rows:48,chunk_width:8,initial_elevation:0,initial_heights:[{patch_rows}],floor:-8,ceiling:8,generation:Version1(bands:[(bottom:-8,top:-2,choices:[(material:"{prefix+binding["ore_material"]}",weight:1)]),(bottom:-2,top:{highest},choices:[(material:"{prefix+binding["material"]}",weight:1)])]))),'
    original=(package/'maps/field.ron').read_text();identity=re.search(r'\bid:\s*"field"',original)
    if identity is None:raise ValueError('cannot retain verified public map appearance')
    appearance=original[1:identity.start()]
    # Keep the public ground material/environment fragment, replacing only the authored world.
    if 'terrain:' in appearance:raise ValueError('unexpected terrain before public map identity')
    put('maps/field.ron',f'({appearance}id:"field",size:(x:64.0,y:48.0),visibility:None,{terrain}placements:[{",".join(rows)}],nodes:[],slot_economy:{accounts})')
    commands=[]
    for command in scene['commands']:
        ids=[mapping[name] for name in command['actors']]
        if args.reverse_members:ids.reverse()
        target='None' if command['action']=='hold' else {'Point':{'x':command['point'][0],'y':command['point'][1]}} if command['action']=='move' else {'TerrainWork':{'tool':prefix+binding['fill' if command['action']=='fill' else 'dig'],'cells':{'min':dict(zip(['x','y'],command['cells']['min'])),'max':dict(zip(['x','y'],command['cells']['max']))},'target_elevation':command['target_elevation']}}
        order=prefix+{'move':'order_move','hold':'order_hold'}.get(command['action'],'order_work')
        commands.append({'tick':command['tick'],'slot':command['slot'],'envelope':{'order':order,'units':ids,'target':target,'queue':command['queue'],'group':command['group']}})
    (out/'commands.json').write_text(json.dumps(commands,indent=2)+'\n')
    put('README.md',f'# Public group fixture: {args.scene}\n\nStatus: AUTHORED_UNRUN. This generated variant has not passed typed archive, simulation or browser checks.\n\nPhysical input comes from scenes.json. Actors: {len(actors)}. Finite commands: {len(commands)}. No private seed or realized columns are in this package.\n\nDetailed model/texture/animation bytes and existing assets/CREDITS.md are reused unchanged from {args.package}. Only public map, footprint/profile, visual scale and tool/resource records are authored for this frozen case.\n')
    frozen={'status':'AUTHORED_UNRUN','scene_name':args.scene,'package':args.package,'tick_hz':20,'scene':scene,'profiles':data['profiles'],'bindings':binding,'actors':[dict(a,unit=mapping[a['label']],content=content(a),profile=profile(data,scene,a)) for a in scene['actors']],'reverse_allocation':args.reverse_allocation,'reverse_members':args.reverse_members,'source_scenes_sha256':sha(SCENES),'commands_sha256':sha(out/'commands.json'),'public_files_sha256':{str(p.relative_to(package)):sha(p) for p in sorted(package.rglob('*')) if p.is_file()},'asset_policy':'unchanged detailed source asset bytes, hardlinked when possible; existing credits retained','pending':['typed archive/staging validation','native swept/trajectory/determinism checks','real browser input','performance and cold-play checks after accepted M4 merge']}
    (out/'fixture.json').write_text(json.dumps(frozen,indent=2)+'\n');print(json.dumps({'status':'AUTHORED_UNRUN','scene':args.scene,'actors':len(actors),'commands':len(commands),'output':str(out)}))


if __name__=='__main__':main()
