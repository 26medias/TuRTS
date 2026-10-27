#!/usr/bin/env python3
"""Compose TuRTS social images from existing artwork; requires Pillow."""
from collections import deque
from pathlib import Path
import json
import zipfile
from PIL import Image, ImageChops, ImageDraw, ImageFilter, ImageFont, ImageOps

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'docs/assets/social'
OUT.mkdir(parents=True, exist_ok=True)
ART = Image.open(ROOT / 'docs/assets/turts-concept-art.png').convert('RGBA')
RAW = Image.open(ROOT / 'docs/assets/turts-title.png').convert('RGBA')
ALPHA = RAW.getchannel('A')
LOGO = RAW.crop(ALPHA.point(lambda a: 255 if a > 64 else 0).getbbox())
FONT = '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'
manifest = []

def background(size, center=(.5,.5), darkness=0):
    im = ImageOps.fit(ART, size, Image.Resampling.LANCZOS, centering=center)
    if darkness:
        im = Image.blend(im, Image.new('RGBA', size, (5,14,25,255)), darkness)
    return im

def place(im, asset, width, y):
    asset = asset.resize((width, round(asset.height*width/asset.width)), Image.Resampling.LANCZOS)
    x = (im.width-width)//2
    alpha = Image.new('L', im.size)
    alpha.paste(asset.getchannel('A'), (x,y+4))
    shadow = Image.new('RGBA', im.size)
    shadow.putalpha(alpha.filter(ImageFilter.GaussianBlur(7)).point(lambda a: round(a*.8)))
    im = Image.alpha_composite(im, shadow)
    im.alpha_composite(asset, (x,y))
    return im, (x,y,x+asset.width,y+asset.height)

def save(im, name, use):
    path = OUT/name
    if path.suffix == '.jpg':
        im.convert('RGB').save(path, quality=94, subsampling=0, optimize=True)
    else:
        im.save(path, optimize=True)
    with Image.open(path) as check:
        assert check.size == im.size
        check.verify()
    manifest.append(dict(file=name,width=im.width,height=im.height,bytes=path.stat().st_size,use=use))

width = 1120
height = round(LOGO.height*width/LOGO.width)
banner,b = place(background((2560,1440),darkness=.08), LOGO, width, (1440-height)//2)
# Conservative central title area fits YouTube's published minimum safe area.
assert b[0] >= (2560-1235)//2 and b[2] <= (2560+1235)//2
assert b[1] >= (1440-338)//2 and b[3] <= (1440+338)//2
save(banner,'youtube-banner-2560x1440.jpg','YouTube channel banner; branding centered for mobile crop')
assert (OUT/manifest[-1]['file']).stat().st_size < 6_000_000
preview=banner.copy()
d=ImageDraw.Draw(preview)
d.rectangle(((2560-1235)//2,(1440-338)//2,(2560+1235)//2,(1440+338)//2),outline='#55d6ff',width=4)
d.text((680,510),'TITLE SAFE AREA / REFERENCE ONLY',font=ImageFont.truetype(FONT,26),fill='white',stroke_width=2,stroke_fill='black')
save(preview,'youtube-safe-area-preview.jpg','Reference only; do not upload')

# Isolate the original first T by following its alpha-connected pixels.
mask=ALPHA.point(lambda a: 255 if a>100 else 0)
px=mask.load()
seen=Image.new('L',mask.size)
sp=seen.load()
q=deque([(240,220)])
sp[240,220]=255
while q:
    x,y=q.popleft()
    for nx,ny in ((x-1,y),(x+1,y),(x,y-1),(x,y+1)):
        if 0<=nx<mask.width and 0<=ny<mask.height and px[nx,ny] and not sp[nx,ny]:
            sp[nx,ny]=255
            q.append((nx,ny))
mark=RAW.copy()
mark.putalpha(ImageChops.multiply(ALPHA,seen.filter(ImageFilter.MaxFilter(5))))
mark=mark.crop(mark.getchannel('A').getbbox())
assert mark.width < RAW.width/2
avatar,_=place(background((1024,1024),(.25,.5),.72),mark,640,235)
for name,size,use in [
    ('avatar-1024.png',1024,'Shared profile image, original T isolated for circular crops'),
    ('youtube-avatar-800.png',800,'YouTube profile picture'),
    ('x-avatar-400.png',400,'X profile picture')]:
    save(avatar.resize((size,size),Image.Resampling.LANCZOS).convert('RGB'),name,use)
watermark=Image.new('RGBA',(150,150))
wm=ImageOps.contain(mark,(116,116),Image.Resampling.LANCZOS)
watermark.alpha_composite(wm,((150-wm.width)//2,(150-wm.height)//2))
save(watermark,'youtube-watermark-150.png','Transparent video watermark')

for name,size,width,y,center,use in [
    ('x-header-1500x500.jpg',(1500,500),820,82,(.5,.4),'X header'),
    ('youtube-thumbnail-1280x720.jpg',(1280,720),790,40,(.5,.5),'Branded concept-art thumbnail'),
    ('social-landscape-1200x675.jpg',(1200,675),760,40,(.5,.5),'Landscape teaser post'),
    ('social-square-1080x1080.jpg',(1080,1080),920,95,(.42,.5),'Square feed teaser'),
    ('instagram-portrait-1080x1350.jpg',(1080,1350),930,125,(.37,.5),'Portrait feed teaser'),
    ('story-1080x1920.jpg',(1080,1920),900,290,(.35,.5),'Vertical artwork for Stories, Reels and TikTok'),
    ('discord-banner-960x540.jpg',(960,540),650,78,(.5,.5),'Optional Discord server banner where available')]:
    im,_=place(background(size,center,.08),LOGO,width,y)
    save(im,name,use)

tiles=[m for m in manifest if 'preview' not in m['file'] and 'watermark' not in m['file'] and m['file'] not in ('youtube-avatar-800.png','x-avatar-400.png')]
sheet=Image.new('RGB',(1440,((len(tiles)+2)//3)*365),'#0b1521')
draw=ImageDraw.Draw(sheet)
font=ImageFont.truetype(FONT,17)
for i,m in enumerate(tiles):
    x,y=(i%3)*480,(i//3)*365
    thumb=Image.open(OUT/m['file']).convert('RGB')
    thumb.thumbnail((452,300),Image.Resampling.LANCZOS)
    sheet.paste(thumb,(x+(480-thumb.width)//2,y+(300-thumb.height)//2+10))
    draw.text((x+14,y+322),m['file'],font=font,fill='#edf2f7')
sheet.save(OUT/'social-kit-preview.jpg',quality=91)
(OUT/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
with zipfile.ZipFile(OUT/'turts-social-kit.zip','w',zipfile.ZIP_DEFLATED) as z:
    for m in manifest:
        if 'preview' not in m['file']:
            z.write(OUT/m['file'],m['file'])
    z.write(OUT/'manifest.json','manifest.json')
print(json.dumps(manifest,indent=2))
