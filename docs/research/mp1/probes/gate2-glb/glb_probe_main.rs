// Probe: the D 8.1 pack-time checks are computable with gltf 1.4.1 (no bevy), on a script-made GLB.
use gltf::{Glb, Gltf};
fn walk(node: gltf::Node, parent: [[f32; 4]; 4], lo: &mut f32, r: &mut f32, tris: &mut usize) {
    let m = mul(parent, node.transform().matrix());
    if let Some(mesh) = node.mesh() {
        for p in mesh.primitives() {
            *tris += p.indices().map_or(0, |a| a.count()) / 3;
            let b = p.bounding_box();
            for &x in &[b.min[0], b.max[0]] { for &y in &[b.min[1], b.max[1]] { for &z in &[b.min[2], b.max[2]] {
                let w = [m[0][0]*x + m[1][0]*y + m[2][0]*z + m[3][0], m[0][1]*x + m[1][1]*y + m[2][1]*z + m[3][1], m[0][2]*x + m[1][2]*y + m[2][2]*z + m[3][2]];
                *lo = lo.min(w[1]); *r = r.max((w[0]*w[0] + w[2]*w[2]).sqrt());
            }}}
        }
    }
    for c in node.children() { walk(c, m, lo, r, tris); }
}
fn mul(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut o = [[0.0; 4]; 4];
    for c in 0..4 { for rr in 0..4 { for k in 0..4 { o[c][rr] += a[k][rr] * b[c][k]; } } }
    o
}
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let glb = Glb::from_slice(&bytes).expect("GLB");
    println!("glb_version={}", glb.header.version);
    let g = Gltf::from_slice(&bytes).expect("parse");
    let d = &g.document;
    let uris = d.buffers().filter(|b| matches!(b.source(), gltf::buffer::Source::Uri(_))).count()
        + d.images().filter(|i| matches!(i.source(), gltf::image::Source::Uri { .. })).count();
    println!("external_uris={uris} extensions_required={:?}", d.extensions_required().collect::<Vec<_>>());
    println!("skins={} animations={} cameras={} materials={}", d.skins().count(), d.animations().count(), d.cameras().count(), d.materials().count());
    println!("material_names={:?}", d.materials().map(|m| m.name().unwrap_or("?").to_string()).collect::<Vec<_>>());
    println!("node_names={:?}", d.nodes().map(|n| n.name().unwrap_or("?").to_string()).collect::<Vec<_>>());
    let (mut lo, mut r, mut tris) = (f32::MAX, 0.0f32, 0usize);
    let id = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
    for s in d.default_scene().into_iter() { for n in s.nodes() { walk(n, id, &mut lo, &mut r, &mut tris); } }
    println!("triangles={tris} min_y={lo:.3} xz_radius={r:.3} bytes={}", bytes.len());
    let fr = 0.9f32;
    println!("pivot_ok={} scale_ok={} tris_ok={} bytes_ok={}", (-0.05..=0.05).contains(&lo), (0.8*fr..=1.2*fr).contains(&r), tris <= 8000, bytes.len() <= 4_194_304);
}
