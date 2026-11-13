import json,math,re,shutil
from pathlib import Path
import argparse
parser=argparse.ArgumentParser(description='Generate P2 walking and visual-body workloads.')
parser.add_argument('output',type=Path)
parser.add_argument('--root',type=Path,default=Path(__file__).resolve().parents[3])
args=parser.parse_args()
root=args.root.resolve();out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
for name,count in [('walk200',200),('walk400',400),('mixed400',400),('walk1000',1000)]:
 p=out/name
 if p.exists(): shutil.rmtree(p)
 shutil.copytree(root/'mods/skirmish',p)
 cols=math.ceil(math.sqrt(count));rows=math.ceil(count/cols);w=cols*2.8+18;h=rows*1.4+4
 placements=[];script=[]
 for i in range(count):
  x=2+i%cols*1.4;y=2+i//cols*1.4
  placements.append(f'(unit:"skirmish:soldier",owner_slot:0,pos:(x:{x:.5f},y:{y:.5f}),contents:[])')
  if name!='mixed400' or i<200:script.append({'tick':300,'slot':0,'envelope':{'order':'skirmish:order_move','units':[i+1],'target':{'Point':{'x':x+cols*1.4+10,'y':y}},'queue':'Replace','group':0}})
 (p/'maps/field.ron').write_text(f'(id:"field",size:(x:{w:.5f},y:{h:.5f}),placements:[{",".join(placements)}],nodes:[],slot_economy:[(slot:0,balances:[(balance:"skirmish:credits",amount:10000)])])')
 (p/'match.ron').write_text('(start:Immediate,victory:None,open_sites_per_slot:Some(4))')
 unit=(p/'units/soldier.ron').read_text().replace('speed: 4.0','speed: 0.4')
 # A rendering fixture: no AI, targeting, or battle simulation in the walking cost rows.
 unit=unit[:unit.index('weapon:')]+'weapon: None, '+unit[unit.index('removal:'):]
 (p/'units/soldier.ron').write_text(unit)
 if name=='mixed400':
  (p/'orders/bench_remove.ron').write_text('(id:"bench_remove",name:"Benchmark removal",behaviour:Remove,target:None,fire:None,engage:None,icon:"icon_order_stop",hotkey:"key_stop",cursor:None,card:None,units:["skirmish:soldier"],description:"Scripted visual body load.")')
  m=(p/'manifest.ron').read_text().replace('orders: [','orders: ["orders/bench_remove.ron",');(p/'manifest.ron').write_text(m)
  for n in range(200):script.append({'tick':400+n,'slot':0,'envelope':{'order':'skirmish:bench_remove','units':[201+n],'target':'None','queue':'Replace','group':0}})
 (out/f'{name}-commands.json').write_text(json.dumps(script))
print(out)
