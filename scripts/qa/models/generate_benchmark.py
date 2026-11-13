#!/usr/bin/env python3
"""Generate reproducible model load fixtures. Art stays outside the shipped skirmish."""
from pathlib import Path
import argparse,json,math,struct,shutil,io
from PIL import Image

def glb(path,lat,lon,textures):
    vertices=[]; normals=[]; uvs=[]; indices=[]
    # A 1.7-unit detailed body. 100 by 100 quads = exactly 20,000 triangles.
    for y in range(lat+1):
        theta=math.pi*y/lat
        for x in range(lon+1):
            angle=2*math.pi*x/lon
            nx,nz=math.sin(theta)*math.cos(angle),math.sin(theta)*math.sin(angle)
            vertices.extend((nx*.48,(math.cos(theta)+1)*.85,nz*.48))
            normals.extend((nx,math.cos(theta),nz));uvs.extend((x/lon,y/lat))
    for y in range(lat):
        for x in range(lon):
            a=y*(lon+1)+x;b=a+lon+1
            indices.extend((a,a+1,b,a+1,b+1,b))
    binary=bytearray();views=[];accessors=[]
    def data(blob):
        while len(binary)%4:binary.append(0)
        index=len(views);views.append({'buffer':0,'byteOffset':len(binary),'byteLength':len(blob)});binary.extend(blob);return index
    def accessor(values,kind,component,fmt,count,**extra):
        i=len(accessors);accessors.append({'bufferView':data(struct.pack('<'+fmt*len(values),*values)),'componentType':component,'count':count,'type':kind,**extra});return i
    pos=accessor(vertices,'VEC3',5126,'f',len(vertices)//3,min=[-.48,0,-.48],max=[.48,1.7,.48])
    norm=accessor(normals,'VEC3',5126,'f',len(normals)//3)
    uv=accessor(uvs,'VEC2',5126,'f',len(uvs)//2)
    idx=accessor(indices,'SCALAR',5125,'I',len(indices))
    images=[]
    for file in textures:images.append({'bufferView':data(file.read_bytes()),'mimeType':'image/png'})
    material={'name':'team','pbrMetallicRoughness':{'baseColorFactor':[1,1,1,1],'metallicFactor':.2,'roughnessFactor':.8}}
    if images:material['pbrMetallicRoughness'].update(baseColorTexture={'index':0},metallicRoughnessTexture={'index':2});material['normalTexture']={'index':1}
    document={'asset':{'version':'2.0'},'scene':0,'scenes':[{'nodes':[0]}],'nodes':[{'name':'body','mesh':0}],'meshes':[{'primitives':[{'attributes':{'POSITION':pos,'NORMAL':norm,'TEXCOORD_0':uv},'indices':idx,'material':0}]}],'materials':[material],'buffers':[{'byteLength':len(binary)}],'bufferViews':views,'accessors':accessors,'images':images,'textures':[{'source':i} for i in range(len(images))]}
    encoded=json.dumps(document,separators=(',',':')).encode();encoded+=b' '*((-len(encoded))%4);binary+=b'\0'*((-len(binary))%4)
    path.write_bytes(struct.pack('<III',0x46546c67,2,12+8+len(encoded)+8+len(binary))+struct.pack('<II',len(encoded),0x4e4f534a)+encoded+struct.pack('<II',len(binary),0x004e4942)+binary)

def main():
    p=argparse.ArgumentParser();p.add_argument('output',type=Path);p.add_argument('--root',type=Path,default=Path(__file__).resolve().parents[3]);args=p.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
    textures=[]
    for name,colour in [('base',(155,145,130,255)),('normal',(128,128,255,255)),('mr',(255,200,50,255))]:
        image=Image.new('RGBA',(2048,2048),colour)
        # Deterministic surface detail prevents the base texture from being a single flat sample.
        if name=='base':
            pixels=image.load()
            for y in range(2048):
                for x in range(2048):
                    n=((x*73+y*151+(x*y)%127)%31)-15;pixels[x,y]=(155+n,145+n,130+n,255)
        target=out/(name+'.png');image.save(target);textures.append(target)
    for name,lat,lon in [('detail',100,100),('lod1',20,20),('lod2',8,8)]:glb(out/(name+'.glb'),lat,lon,textures)
    for name,count in [('shapes',400),('lowpoly',400),('detail',400),('detail_lod',400),('detail_lod_2000',2000)]:
        package=out/name
        if package.exists():shutil.rmtree(package)
        shutil.copytree(args.root/'mods/skirmish',package)
        manifest=(package/'manifest.ron').read_text().replace('^0.2.4','^0.2.5')
        (package/'manifest.ron').write_text(manifest)
        modelassets=[]
        if name=='lowpoly':
            shutil.copy2(args.root/'mods/skirmish/assets/models/soldier.glb',package/'assets/soldier.glb');modelassets=[('bench','soldier.glb')]
        elif name!='shapes':
            for asset in ['detail','lod1','lod2']:shutil.copy2(out/(asset+'.glb'),package/'assets'/(asset+'.glb'));modelassets.append((asset,asset+'.glb'))
        assets=(package/'assets/assets.ron').read_text().rstrip().removesuffix(')')
        assets+=', models: ['+','.join(f'(id: "{id}", path: "assets/{file}")'for id,file in modelassets)+'])\n';(package/'assets/assets.ron').write_text(assets)
        if modelassets:
            model='bench'if name=='lowpoly'else'detail';lods='[(asset: "skirmish:lod1", below_screen_height: 0.08),(asset: "skirmish:lod2", below_screen_height: 0.03)]'if 'lod'in name else'[]'
            (package/'visuals/soldier.ron').write_text(f'(target: Unit("skirmish:soldier"), shape: Capsule(radius: 0.4, height: 1.6), model: Some((asset: "skirmish:{model}", team_material: Some("team"), lods: {lods}, scale: 1.0)))\n')
        # Idle units isolate client model costs. No computer runs in these fixtures.
        cols=math.ceil(math.sqrt(count));rows=math.ceil(count/cols);width=cols*1.4+4;depth=rows*1.4+4
        placements=','.join(f'(unit: "skirmish:soldier", owner_slot: 0, pos: (x: {2+x%cols*1.4:.4f}, y: {2+x//cols*1.4:.4f}), contents: [])'for x in range(count))
        (package/'maps/field.ron').write_text(f'(id: "field", size: (x: {width:.4f}, y: {depth:.4f}), placements: [{placements}], nodes: [], slot_economy: [(slot: 0, balances: [(balance: "skirmish:credits", amount: 10000)])])\n')
        # Remove win/loss rules: this is a rendering fixture, not a skirmish match.
        match=(package/'match.ron').read_text();import re
        match=re.sub(r'victory: Some\(.*', 'victory: None,)',match,flags=re.S)
        # Preserve valid match syntax through an explicit fixture definition.
        (package/'match.ron').write_text('(start: Immediate, victory: None)\n')
    print(out)
if __name__=='__main__':main()
