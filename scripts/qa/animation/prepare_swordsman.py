"""Prepare the CC0 Knight sword and shield. Arguments: source directory, output directory."""
import bpy,sys,os,math,mathutils
SRC,OUT=sys.argv[-2],sys.argv[-1]
V=mathutils.Vector
def reset(): bpy.ops.wm.read_factory_settings(use_empty=True)
def imp(f): bpy.ops.import_scene.gltf(filepath=os.path.join(SRC,f))
def meshes(): return [o for o in bpy.context.scene.objects if o.type=="MESH"]
def bounds(objs):
    mn=V((1e9,)*3); mx=V((-1e9,)*3)
    dg=bpy.context.evaluated_depsgraph_get()
    for o in objs:
        e=o.evaluated_get(dg); me=e.to_mesh()
        for v in me.vertices:
            w=e.matrix_world@v.co
            for k in range(3): mn[k]=min(mn[k],w[k]); mx[k]=max(mx[k],w[k])
        e.to_mesh_clear()
    return mn,mx
def team_mat():
    m=bpy.data.materials.get("team") or bpy.data.materials.new("team")
    m.use_nodes=True
    b=next(n for n in m.node_tree.nodes if n.type=="BSDF_PRINCIPLED")
    b.inputs["Base Color"].default_value=(0.92,0.92,0.92,1); b.inputs["Roughness"].default_value=0.7
    return m
def split_team_by_texture(o,pred):
    me=o.data
    if not me.materials or not me.uv_layers: return 0
    mat=me.materials[0]
    img=next((n.image for n in mat.node_tree.nodes if n.type=="TEX_IMAGE" and n.image),None)
    if not img: return 0
    w,h=img.size; px=img.pixels[:]
    tm=team_mat()
    if "team" not in [m.name for m in me.materials]: me.materials.append(tm)
    ti=[m.name for m in me.materials].index("team")
    uv=me.uv_layers.active.data; n=0
    for p in me.polygons:
        u=sum(uv[i].uv[0] for i in p.loop_indices)/p.loop_total; v=sum(uv[i].uv[1] for i in p.loop_indices)/p.loop_total
        x=int((u%1.0)*w)%w; y=int((v%1.0)*h)%h; k=(y*w+x)*4
        if pred(px[k],px[k+1],px[k+2]): p.material_index=ti; n+=1
    return n
def export(name,anim):
    bpy.ops.export_scene.gltf(filepath=os.path.join(OUT,name+".glb"),export_format="GLB",export_animations=anim,export_yup=True,export_apply=not anim,export_image_format="AUTO")
    print("OUT",name,os.path.getsize(os.path.join(OUT,name+".glb")))
def character(f,name,keep,height,pose_action,pose_frame,clips,pred,yaw=0.0):
    reset(); imp(f)
    arm=next(o for o in bpy.context.scene.objects if o.type=="ARMATURE")
    for o in list(meshes()):
        if not any(o.name == k for k in keep): bpy.data.objects.remove(o,do_unlink=True)
    # Keep the blade inside the selectable footprint, as for the crossbow prop.
    sword=bpy.data.objects["1H_Sword"]
    sword.scale=tuple(value*0.6 for value in sword.scale)
    nteam=sum(split_team_by_texture(o,pred) for o in meshes())
    print("TEAMFACES",name,nteam,"of",sum(len(o.data.polygons) for o in meshes()))
    # pose
    arm.animation_data_create()
    for t in list(arm.animation_data.nla_tracks): arm.animation_data.nla_tracks.remove(t)
    act=bpy.data.actions[pose_action]; arm.animation_data.action=act
    try:
        if hasattr(arm.animation_data,"action_slot") and act.slots: arm.animation_data.action_slot=act.slots[0]
    except Exception as e: print("slot",e)
    bpy.context.scene.frame_set(pose_frame); bpy.context.view_layer.update()
    mn,mx=bounds(meshes()); s=height/(mx.z-mn.z)
    print("POSEBOUNDS",name,[round(v,2) for v in mn],[round(v,2) for v in mx],"scale",round(s,3))
    # static bake
    dg=bpy.context.evaluated_depsgraph_get(); baked=[]
    for o in meshes():
        e=o.evaluated_get(dg); me=bpy.data.meshes.new_from_object(e)
        me.transform(mathutils.Matrix.Rotation(math.radians(yaw),4,'Z')@mathutils.Matrix.Scale(s,4)@mathutils.Matrix.Translation(V((0,0,-mn.z)))@e.matrix_world)
        baked.append((o.name,me))
    # animated export first
    arm.animation_data.action=None
    for a in list(bpy.data.actions):
        if a.name not in clips: bpy.data.actions.remove(a)
    arm.scale=(s,s,s)
    bpy.context.view_layer.update()
    export(name+"_anim",True)
    # static scene
    for o in list(bpy.context.scene.objects): bpy.data.objects.remove(o,do_unlink=True)
    objs=[]
    for n,me in baked:
        o=bpy.data.objects.new(n,me); bpy.context.scene.collection.objects.link(o); objs.append(o)
    bpy.context.view_layer.objects.active=objs[0]
    for o in objs: o.select_set(True)
    bpy.ops.object.join(); o=bpy.context.view_layer.objects.active; o.name=name
    mn,mx=bounds([o]); print("BOUNDS",name,[round(v,2) for v in mn],[round(v,2) for v in mx],"mats",[m.name for m in o.data.materials])
    export(name,False)
red=lambda r,g,b: r>0.35 and r>1.6*g and r>1.6*b
CL=["Idle","Walking_A","Running_A","Death_A","Interact","1H_Melee_Attack_Chop","Hit_A"]
character("Knight.glb","swordsman",["Knight_ArmLeft","Knight_ArmRight","Knight_Body","Knight_Head","Knight_LegLeft","Knight_LegRight","Knight_Helmet","Knight_Cape","1H_Sword","Round_Shield"],1.7,"Idle",1,CL,red)
