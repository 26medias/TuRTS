# gate1-bevy evidence
## png probe
file_bytes=19771 decoded_bytes=16777216 images_in_16MiB=848 decoded_total_GiB=13.2
## cursor/pointer-lock probe (Chrome 156 headless, Playwright)
{"red_pixels_in_screenshot":0,"locked":true,"lockErr":null,"moves":[[100,100,0,0],[100,100,50,20],[100,100,249,0]]}
## bevy_audio resume grep (expect 0 lines)
## cpal resume
430:        match self.ctx.resume() {
464:        match self.ctx.suspend() {
## bevy features
audio = [
    "bevy_audio",
    "vorbis",
]
45:vorbis = ["rodio/lewton"]
{"red_pixels_in_screenshot":1024,"locked":true,"lockErr":null,"moves":[[100,100,0,0],[100,100,50,20],[100,100,249,0]]}
noaudio raw=56371295 bindgen=62113976 wasm_opt=23565331 brotli=5109644
audio raw=57351049 bindgen=63175715 wasm_opt=23977635 brotli=5177671
