#!/usr/bin/env python3
"""Use the sealed bundle's actual package identity without bypassing mc_prepare checks."""
import argparse,hashlib,json,os,subprocess,sys
from pathlib import Path


def prepared_package(prepared,root):
    manifest=json.loads((prepared/'manifest.json').read_text())
    package=manifest.get('package')
    if not isinstance(package,str):raise ValueError('prepared manifest lacks package identity')
    relative=Path(package)
    if relative.is_absolute() or '..' in relative.parts or relative.parts[:1]!=('mods',):
        raise ValueError('prepared package is not a repository public mod path')
    candidate=(root/relative).resolve()
    if not candidate.is_relative_to((root/'mods').resolve()) or not candidate.is_dir():
        raise ValueError('prepared public package folder is absent/outside mods')
    return package


def public_hashes(folder):
    result={}
    for current,dirs,files in os.walk(folder):
        if '.claude' in dirs or 'CLAUDE.md' in files:raise ValueError('unexpected non-public configuration in fixture')
        dirs.sort()
        if any((Path(current)/name).is_symlink() for name in dirs):raise ValueError('public fixture must not link directories')
        for name in sorted(files):
            file=Path(current)/name
            if file.is_symlink():raise ValueError('public fixture must contain regular files')
            result[str(file.relative_to(folder))]=hashlib.sha256(file.read_bytes()).hexdigest()
    return result


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('prepared',type=Path,nargs='?');parser.add_argument('--root',type=Path)
    parser.add_argument('--fixture',type=Path);parser.add_argument('--record',type=Path)
    parser.add_argument('--verify-copy',type=Path);parser.add_argument('--copy',type=Path)
    args=parser.parse_args()
    if args.verify_copy:
        record=json.loads(args.verify_copy.read_text())
        if public_hashes(args.copy)!=record['public_files_sha256']:raise ValueError('copied terrain fixture differs from the checked source')
        return 0
    root=args.root.resolve();package=prepared_package(args.prepared,root)
    # Keep the existing source, original package, environment, tool and every sealed artifact check.
    command=[sys.executable,'-B',str(root/'scripts/mc_prepare.py'),'check',str(args.prepared),'--root',str(root),'--package',package,'--need','bin/turts_server','--need','bin/turts-pack','--need','dist/index.html']
    checked=subprocess.run(command,check=False)
    if checked.returncode:return checked.returncode
    if args.fixture:
        fixture=args.fixture.resolve()
        if not fixture.is_relative_to((root/'mods').resolve()):raise ValueError('functional fixture must come from repository mods')
        relative=str(fixture.relative_to(root))
        if subprocess.run(['git','-C',str(root),'diff','--quiet','HEAD','--',relative],check=False).returncode:
            raise ValueError('terrain fixture has uncommitted content changes')
        hashes=public_hashes(fixture)
        tracked=subprocess.check_output(['git','-C',str(root),'ls-files','-z','--cached','--',relative]).decode().split('\0')
        tracked={str(Path(name).relative_to(relative)) for name in tracked if name}
        if tracked!=set(hashes):raise ValueError('terrain fixture contains untracked/ignored or missing public files')
        if args.record.exists():raise ValueError('source record already exists; use a fresh output directory')
        record={'prepared_package':package,'fixture':relative,'source_head':subprocess.check_output(['git','-C',str(root),'rev-parse','HEAD'],text=True).strip(),'public_files_sha256':hashes}
        args.record.parent.mkdir(parents=True,exist_ok=True)
        args.record.write_text(json.dumps(record,indent=2)+'\n')
    return 0


if __name__=='__main__':
    try:sys.exit(main())
    except (ValueError,OSError,KeyError) as error:
        print(f'terrain prepared check: {error}',file=sys.stderr);sys.exit(66)
