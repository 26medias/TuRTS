import json,struct
from pathlib import Path
import numpy as np
from scipy.spatial.transform import Rotation,Slerp
import argparse
parser=argparse.ArgumentParser(description='Measure in-place walking stride from glTF foot motion. Requires NumPy and SciPy.')
parser.add_argument('--art',type=Path,default=Path(__file__).resolve().parents[3]/'target/mc-evidence/art/skirmish-fantasy')
parser.add_argument('--out',type=Path,default=Path(__file__).resolve().parents[3]/'target/mc-evidence/p2-stride.json')
parser.add_argument('--model',default=None,help='Measure one model file instead of the original pair')
parser.add_argument('--scale',type=float,default=1.0)
args=parser.parse_args()
base=args.art
out={}
for filename,scale in ([(args.model,args.scale)] if args.model else [('soldier_anim.glb',.88),('worker_anim.glb',1.)]):
 b=(base/filename).read_bytes();n=struct.unpack_from('<I',b,12)[0];d=json.loads(b[20:20+n]);blob=b[28+n:]
 def acc(i):
  a=d['accessors'][i];v=d['bufferViews'][a['bufferView']];nc={'SCALAR':1,'VEC3':3,'VEC4':4}[a['type']];return np.frombuffer(blob,dtype='<f4',count=a['count']*nc,offset=v.get('byteOffset',0)+a.get('byteOffset',0)).reshape(a['count'],nc)
 a=next(a for a in d['animations'] if a['name']=='Walking_A');duration=max(acc(s['input'])[-1,0] for s in a['samplers']);parents={c:i for i,n in enumerate(d['nodes']) for c in n.get('children',[])};feet=[i for i,n in enumerate(d['nodes']) if n.get('name') in ['foot.l','foot.r']];series={i:[] for i in feet}
 for t in np.linspace(0,float(duration),241):
  transforms=[{k:np.array(n.get(k,v),dtype=float) for k,v in [('translation',[0,0,0]),('rotation',[0,0,0,1]),('scale',[1,1,1])]} for n in d['nodes']]
  for c in a['channels']:
   s=a['samplers'][c['sampler']];times=acc(s['input'])[:,0];values=acc(s['output']);k=c['target']['path'];idx=c['target']['node'];ct=min(max(t,times[0]),times[-1])
   if k=='rotation': v=Slerp(times,Rotation.from_quat(values))([ct]).as_quat()[0]
   else: v=np.array([np.interp(ct,times,values[:,j]) for j in range(values.shape[1])])
   transforms[idx][k]=v
  matrices=[]
  for x in transforms:
   m=np.eye(4);m[:3,:3]=Rotation.from_quat(x['rotation']).as_matrix()@np.diag(x['scale']);m[:3,3]=x['translation'];matrices.append(m)
  def world(i):return (world(parents[i])@matrices[i]) if i in parents else matrices[i]
  for i in feet:series[i].append((world(i)[:3,3]*scale).tolist())
 estimates=[];samples={}
 for i,ps in series.items():
  p=np.array(ps);dt=duration/240;vel=np.diff(p,axis=0)/dt;height=p[:-1,1];contact=(height<height.min()+.025)&(vel[:,2]<0)
  # During ground contact, the backward foot speed cancels forward body speed.
  speed=float(np.median(-vel[contact,2]));stride=speed*float(duration);estimates.append(stride);samples[d['nodes'][i]['name']]={'contact_samples':int(contact.sum()),'speed':speed,'stride':stride,'height_range':[float(p[:,1].min()),float(p[:,1].max())],'z_range':[float(p[:,2].min()),float(p[:,2].max())]}
 out[filename]={'method':'241 glTF pose samples; median backward foot speed within 0.025 map units of lowest foot joint, multiplied by clip duration','scale':scale,'duration':float(duration),'stride':float(np.mean(estimates)),'feet':samples}
p=args.out;p.write_text(json.dumps(out,indent=2));print(p.read_text())
