#!/usr/bin/env python3
"""Freeze the supplied Idle clip for the AN7 pixel control. This edits only a test copy."""
import json,struct,sys
from pathlib import Path
p=Path(sys.argv[1]);b=bytearray(p.read_bytes());length=struct.unpack_from('<I',b,12)[0];d=json.loads(b[20:20+length]);animation=next(a for a in d['animations'] if a['name']=='Idle')
for sampler in animation['samplers']:
 a=d['accessors'][sampler['output']];v=d['bufferViews'][a['bufferView']];size={'SCALAR':1,'VEC3':3,'VEC4':4}[a['type']]*4
 offset=28+length+v.get('byteOffset',0)+a.get('byteOffset',0);stride=v.get('byteStride',size);first=bytes(b[offset:offset+size])
 for i in range(a['count']):b[offset+i*stride:offset+i*stride+size]=first
p.write_bytes(b)
