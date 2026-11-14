"""Generate sustained combat render workloads, without computer strategy overhead."""
import argparse,json,math,shutil,re
from pathlib import Path
parser=argparse.ArgumentParser()
parser.add_argument('output',type=Path)
parser.add_argument('--root',type=Path,default=Path(__file__).resolve().parents[3])
parser.add_argument('--units',type=int,nargs='+',help='Requested actor counts, including 2000')
parser.add_argument('--package',type=Path,help='Source package; defaults to ROOT/mods/skirmish')
parser.add_argument('--presentation',type=Path,help='RON field fragment from the map_presentation example')
parser.add_argument('--mixed',action='store_true',help='Mixed melee and ranged pairs with shipped weapon reach')
args=parser.parse_args();root=args.root.resolve();out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
scenes=[('battle200',100,0),('burning200',100,10),('battle400',200,0),('battle800',400,0)]
if args.units and 2000 in args.units: scenes.append(('battle2000',1000,0))
presentation=args.presentation.read_text() if args.presentation else ''
for name,side,burning in scenes:
    if args.units and side*2+burning not in args.units: continue
    if args.mixed and (burning or (not args.units and name!='battle400')): continue
    package=out/name
    if package.exists():shutil.rmtree(package)
    shutil.copytree(args.package or root/'mods/skirmish',package)
    cols=math.ceil(math.sqrt(side));rows=math.ceil(side/cols);offset=cols*1.4+4
    width=offset+cols*1.4+4;height=max(rows*1.4+4,20)
    if args.mixed: width=cols*3+4; height=max(rows*1.4+4,20)
    placements=[];commands=[]
    for slot in range(2):
        for i in range(side):
            x=2+i%cols*1.4+slot*offset;y=2+i//cols*1.4
            kind='soldier'
            if args.mixed: x=2+i%cols*3+slot*1.7; kind='swordsman' if i%2==0 else 'soldier'
            placements.append(f'(unit:"skirmish:{kind}",owner_slot:{slot},pos:(x:{x:.5f},y:{y:.5f}),contents:[])')
            commands.append({'tick':400 if burning else 30,'slot':slot,'envelope':{'order':'skirmish:order_attack','units':[slot*side+i+1],'target':{'Unit':(1-slot)*side+i+1},'queue':'Replace','group':0}})
    for i in range(burning):
        placements.append(f'(unit:"skirmish:barracks",owner_slot:0,pos:(x:{4+i*4:.5f},y:{height+4:.5f}),contents:[])')
    (package/'maps/field.ron').write_text(f'({presentation}id:"field",size:(x:{max(width,50):.5f},y:{height+10:.5f}),placements:[{",".join(placements)}],nodes:[],slot_economy:[(slot:0,balances:[(balance:"skirmish:credits",amount:10000)])])')
    hud=package/'interface/hud.ron'
    camera=hud.read_text().replace('start: OwnedCentre','start: MapCentre')
    camera=re.sub(r'height: 36.0',f'height: {(width+height+10)*0.8:.5f}',camera).replace('maximum: 100.0','maximum: 300.0')
    hud.write_text(camera)
    (package/'match.ron').write_text('(start:Immediate,victory:None,open_sites_per_slot:Some(4))')
    text=(package/'units/soldier.ron').read_text().replace('max: 120','max: 1000000')
    if not args.mixed: text=text.replace('range: 6.0',f'range: {100 if burning else offset+1:.5f}')
    (package/'units/soldier.ron').write_text(text)
    if args.mixed:
        path=package/'units/swordsman.ron';path.write_text(path.read_text().replace('max: 300','max: 1000000'))
    # Isolate the render workload: high building health prevents incidental target changes
    # from killing a building. The damage thresholds are 1/1 in this fixture only.
    # The separate gameplay acceptance probe checks the shipped 1/2 and 1/4 thresholds.
    if burning:
        path=package/'units/barracks.ron'
        path.write_text(path.read_text().replace('max: 500','max: 1000000'))
        path=package/'visuals/barracks.ron'
        path.write_text(path.read_text().replace('numerator:1,denominator:2','numerator:1,denominator:1').replace('numerator:1,denominator:4','numerator:1,denominator:1'))
        for i in range(10):
            commands.append({'tick':10,'slot':1,'envelope':{'order':'skirmish:order_attack','units':[side+i+1],'target':{'Unit':2*side+i+1},'queue':'Replace','group':0}})
            commands.append({'tick':380,'slot':1,'envelope':{'order':'skirmish:order_stop','units':[side+i+1],'target':'None','queue':'Replace','group':0}})
    commands.sort(key=lambda step:step['tick'])
    (out/(name+'-commands.json')).write_text(json.dumps(commands,indent=2))
