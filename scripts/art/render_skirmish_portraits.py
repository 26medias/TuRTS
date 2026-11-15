"""Render existing CC0 Skirmish models. Run with Blender -b -t 4 -P SCRIPT -- MOD_ROOT."""
import math
import sys
from pathlib import Path

import bpy
from mathutils import Vector

root = Path(sys.argv[-1]).resolve()
for name in ('base', 'barracks', 'worker', 'soldier', 'swordsman'):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(root / 'assets/models' / (name + '.glb')))
    scene = bpy.context.scene
    meshes = [obj for obj in scene.objects if obj.type == 'MESH']
    corners = [obj.matrix_world @ Vector(corner) for obj in meshes for corner in obj.bound_box]
    low = Vector(tuple(min(p[i] for p in corners) for i in range(3)))
    high = Vector(tuple(max(p[i] for p in corners) for i in range(3)))
    centre = (low + high) / 2
    extent = max(high - low)
    for mat in bpy.data.materials:
        if mat.name == 'team' and mat.use_nodes:
            next(n for n in mat.node_tree.nodes if n.type == 'BSDF_PRINCIPLED').inputs['Base Color'].default_value = (0.15, 0.38, 0.8, 1)
    bpy.ops.object.camera_add(location=centre + Vector((1.2, -2.0, 1.15)) * extent)
    camera = bpy.context.object
    camera.rotation_euler = (centre - camera.location).to_track_quat('-Z', 'Y').to_euler()
    camera.data.type = 'ORTHO'
    camera.data.ortho_scale = extent * 1.3
    scene.camera = camera
    for offset, power, size in [((2, -3, 4), 700, 4), ((-3, -1, 2), 400, 3)]:
        bpy.ops.object.light_add(type='AREA', location=centre + Vector(offset) * extent)
        lamp = bpy.context.object
        lamp.data.energy = power * extent * extent
        lamp.data.shape = 'DISK'
        lamp.data.size = size * extent
        lamp.rotation_euler = (centre - lamp.location).to_track_quat('-Z', 'Y').to_euler()
    scene.world = bpy.data.worlds.new('Portrait ambient')
    scene.world.use_nodes = True
    scene.world.node_tree.nodes['Background'].inputs[0].default_value = (0.3, 0.35, 0.4, 1)
    scene.world.node_tree.nodes['Background'].inputs[1].default_value = 0.5
    scene.render.engine = 'CYCLES'
    scene.cycles.samples = 24
    scene.cycles.use_denoising = True
    scene.render.resolution_x = scene.render.resolution_y = 256
    scene.render.resolution_percentage = 100
    scene.render.film_transparent = True
    scene.render.image_settings.file_format = 'PNG'
    scene.render.image_settings.color_mode = 'RGBA'
    scene.render.filepath = str(root / 'assets/portraits' / (name + '.png'))
    bpy.ops.render.render(write_still=True)
