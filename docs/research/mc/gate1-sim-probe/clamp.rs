fn main(){ let w=32.0f32; let r=0.5f32; for x in [f32::NAN, f32::INFINITY, -1e9, 40.0] { println!("{x} -> {}", x.clamp(r, w-r)); } }
