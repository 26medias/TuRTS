#!/usr/bin/env bash
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
BROWSER=${1:?usage: run.sh chrome|firefox control|keys}
MODE=${2:?usage: run.sh chrome|firefox control|keys}
case "$BROWSER" in
  chrome) BIN=$(command -v google-chrome || command -v google-chrome-stable || true); CLASS=google-chrome ;;
  firefox) BIN=$(command -v firefox || true); CLASS=firefox ;;
  *) echo "unknown browser: $BROWSER" >&2; exit 2 ;;
esac
[[ -n "$BIN" ]] || { echo "browser binary unavailable: $BROWSER" >&2; exit 3; }
DISPLAY_NUM=:91
export DISPLAY=$DISPLAY_NUM
EVIDENCE=${P1_EVIDENCE_DIR:-/tmp/turts-mc-p1}
mkdir -p "$EVIDENCE"
XVFB_PID= SERVER_PID= BROWSER_PID= WIN= GROUP= RUN_NO=0 RUN_ID=
cleanup() {
  if [[ -n "$BROWSER_PID" ]]; then kill -TERM -- "-$BROWSER_PID" 2>/dev/null || kill "$BROWSER_PID" 2>/dev/null || true; fi
  [[ -n "$SERVER_PID" ]] && kill "$SERVER_PID" 2>/dev/null || true
  [[ -n "$XVFB_PID" ]] && kill "$XVFB_PID" 2>/dev/null || true
}
trap cleanup EXIT
Xvfb "$DISPLAY_NUM" -screen 0 1280x900x24 -nolisten tcp >"$EVIDENCE/xvfb.log" 2>&1 & XVFB_PID=$!
for _ in $(seq 1 50); do xdpyinfo -display "$DISPLAY_NUM" >/dev/null 2>&1 && break; sleep .1; done
python3 "$HERE/server.py" >"$EVIDENCE/server.log" 2>&1 & SERVER_PID=$!
for _ in $(seq 1 50); do curl -fsS http://127.0.0.1:8765/ >/dev/null 2>&1 && break; sleep .1; done
[[ -f "$HERE/p1-log.jsonl" ]] || : > "$HERE/p1-log.jsonl"
OBS="$EVIDENCE/${BROWSER}-${MODE}-observations.jsonl"
: > "$OBS"
append() { python3 - "$HERE/p1-log.jsonl" "$@" <<'PY'
import json,sys,datetime
path,*fields=sys.argv[1:]
row={"type":"probe-observation","at":datetime.datetime.now(datetime.timezone.utc).isoformat(),"browser":fields[0],"group":fields[1],"action":fields[2],"browserActed":fields[3],"evidence":fields[4],"runId":fields[5] if len(fields)>5 else None}
with open(path,"a",encoding="utf-8") as f:f.write(json.dumps(row,separators=(",",":"),sort_keys=True)+"\n")
PY
}
start_browser() {
  GROUP=$1
  RUN_ID="$BROWSER-$GROUP-$$-$(date +%s)"
  local profile="$EVIDENCE/profile-${BROWSER}-${GROUP}-$$"
  mkdir -p "$profile"
  local extras=
  [[ "$GROUP" == locked_esc ]] && extras='&lock=1'
  if [[ "$BROWSER" == chrome ]]; then
    setsid "$BIN" --user-data-dir="$profile" --no-first-run --no-default-browser-check --disable-infobars --new-window "http://127.0.0.1:8765/?browser=$BROWSER&group=$GROUP&runId=$RUN_ID$extras" >"$EVIDENCE/browser-${BROWSER}-${GROUP}.log" 2>&1 &
  else
    setsid "$BIN" --no-remote --profile "$profile" "http://127.0.0.1:8765/?browser=$BROWSER&group=$GROUP&runId=$RUN_ID$extras" >"$EVIDENCE/browser-${BROWSER}-${GROUP}.log" 2>&1 &
  fi
  BROWSER_PID=$!
  WIN=
  for _ in $(seq 1 100); do
    WIN=$(xdotool search --onlyvisible --class "$CLASS" 2>/dev/null | tail -n1 || true)
    [[ -n "$WIN" ]] && break
    sleep .1
  done
  [[ -n "$WIN" ]] || { echo "no headed browser window for $GROUP" >&2; return 10; }
  xdotool windowfocus --sync "$WIN" || true
  sleep 1
}
stop_browser() {
  if [[ -n "$BROWSER_PID" ]]; then
    kill -TERM -- "-$BROWSER_PID" 2>/dev/null || true
    for _ in $(seq 1 40); do kill -0 "$BROWSER_PID" 2>/dev/null || break; sleep .1; done
    kill -KILL -- "-$BROWSER_PID" 2>/dev/null || true
    wait "$BROWSER_PID" 2>/dev/null || true
  fi
  BROWSER_PID=
  sleep .3
}
title() { xdotool getwindowname "$WIN" 2>/dev/null || echo CLOSED; }
snapshot() {
  local out="$EVIDENCE/${BROWSER}-${GROUP}-$(date +%s%N).png"
  import -display "$DISPLAY_NUM" -window root "$out" 2>/dev/null || true
  printf '%s' "$out"
}
window_count() { xdotool search --onlyvisible --class "$CLASS" 2>/dev/null | wc -l; }
ocr() { tesseract "$1" stdout 2>/dev/null | tr '\n' ' ' || true; }
observe_key() {
  local label=$1 sequence=$2 before_title=$3 before_screen=$4 after_title after_screen after_ocr act
  before_count=$(window_count)
  before_focus=$(xdotool getwindowfocus 2>/dev/null || true)
  xdotool key --clearmodifiers "$sequence"
  sleep .45
  after_title=$(title)
  after_count=$(window_count)
  after_focus=$(xdotool getwindowfocus 2>/dev/null || true)
  after_screen=$(snapshot)
  after_ocr=$(ocr "$after_screen")
  act=no
  if [[ "$after_title" != "$before_title" ]]; then act=yes; fi
  if [[ "$after_count" != "$before_count" || "$after_focus" != "$before_focus" ]]; then act=yes; fi
  if [[ "$label" == F10 ]] && [[ "$after_ocr" == *"File"* || "$after_ocr" == *"Edit"* || "$after_ocr" == *"View"* ]]; then act=yes; fi
  if [[ "$label" == F11 ]]; then
    before_dim=$(identify -format '%wx%h' "$before_screen" 2>/dev/null || true)
    after_dim=$(identify -format '%wx%h' "$after_screen" 2>/dev/null || true)
    [[ "$before_dim" != "$after_dim" ]] && act=yes
  fi
  code=$(python3 - "$label" <<'PY'
import sys
k=sys.argv[1]
if k.startswith('Ctrl+') or k.startswith('Shift+'): k=k.split('+')[-1]
if k.startswith('Digit') or k.startswith('Arrow'): print(k); raise SystemExit
if k=='Space': print('Space'); raise SystemExit
if k=='Esc': print('Escape'); raise SystemExit
print(k)
PY
)
  settled=$(python3 - "$HERE/p1-log.jsonl" "$BROWSER" "$GROUP" "$RUN_ID" "$code" "$label" <<'PY'
import json,sys
path,browser,group,run_id,code,label=sys.argv[1:]
want_ctrl=label.startswith('Ctrl+'); want_shift=label.startswith('Shift+')
rows=[]
for line in open(path,encoding='utf-8'):
 row=json.loads(line)
 if row.get('browser')==browser and row.get('group')==group and row.get('runId')==run_id and row.get('code')==code and bool(row.get('ctrl'))==want_ctrl and bool(row.get('shift'))==want_shift and row.get('type') in ('keydown','keydown-settled'):
  rows.append(row)
pre=next((r for r in rows if r.get('type')=='keydown'),None)
post=next((r for r in reversed(rows) if r.get('type')=='keydown-settled'),None)
if not pre or not post: print('NOT OBSERVED')
else:
 changed=(pre.get('activeElement')!=post.get('activeElement') or pre.get('scrollY')!=post.get('scrollY') or pre.get('pointerLocked')!=post.get('pointerLocked'))
 print(('changed' if changed else 'unchanged')+f" active={pre.get('activeElement')}->{post.get('activeElement')} scrollY={pre.get('scrollY')}->{post.get('scrollY')} lock={pre.get('pointerLocked')}->{post.get('pointerLocked')}")
PY
)
  [[ "$settled" == changed* ]] && act=yes
  if [[ "$label" == Esc && "$GROUP" == locked_esc ]]; then
    lock_transition=$(python3 - "$HERE/p1-log.jsonl" "$RUN_ID" <<'PY'
import json,sys
rows=[json.loads(line) for line in open(sys.argv[1],encoding='utf-8')]
rows=[r for r in rows if r.get('type')=='pointerlockchange' and r.get('runId')==sys.argv[2]]
print('true->false' if len(rows)>=2 and rows[-2].get('pointerLocked') is True and rows[-1].get('pointerLocked') is False else 'unobserved')
PY
)
    [[ "$lock_transition" == 'true->false' ]] && act=yes
    settled="pointerLock=$lock_transition; $settled"
  fi
  python3 - "$OBS" "$BROWSER" "$GROUP" "$RUN_ID" "$label" "$sequence" "$before_title" "$after_title" "$before_screen" "$after_screen" "$after_ocr" "$act" "$settled" <<'PY'
import json,sys,datetime
_,path,browser,group,run_id,key,sequence,before,after,shot0,shot1,ocr,acted,settled=sys.argv
row={"at":datetime.datetime.now(datetime.timezone.utc).isoformat(),"browser":browser,"group":group,"runId":run_id,"key":key,"sequence":sequence,"beforeTitle":before,"afterTitle":after,"beforeScreenshot":shot0,"afterScreenshot":shot1,"afterOCR":ocr,"postEventState":settled,"browserActed":acted}
with open(path,"a",encoding="utf-8") as f:f.write(json.dumps(row,separators=(",",":"),sort_keys=True)+"\n")
PY
reaches=$(python3 - "$HERE/p1-log.jsonl" "$BROWSER" "$GROUP" "$RUN_ID" "$code" "$label" <<'PY'
import json,sys
path,browser,group,run_id,code,label=sys.argv[1:]
want_ctrl=label.startswith('Ctrl+')
want_shift=label.startswith('Shift+')
found=False
try:
 for line in open(path,encoding='utf-8'):
  row=json.loads(line)
  if row.get('type')=='keydown' and row.get('browser')==browser and row.get('group')==group and row.get('runId')==run_id and row.get('code')==code and bool(row.get('ctrl'))==want_ctrl and bool(row.get('shift'))==want_shift:
   found=True
except FileNotFoundError: pass
print('yes' if found else 'no')
PY
)
  append "$BROWSER" "$GROUP" "$label" "$act" "reachesPage=$reaches; title: $before_title -> $after_title; windows: $before_count -> $after_count; focus: $before_focus -> $after_focus; state=$settled; screenshot: $before_screen / $after_screen; OCR: $after_ocr" "$RUN_ID"
  echo "$BROWSER $GROUP $label: browser acted=$act; title='$before_title' -> '$after_title'; state='$settled'; OCR='$after_ocr'"
  if [[ "$label" == Ctrl+Digit* || "$label" == Shift+Digit* || "$label" == Digit* || "$label" == F1 || "$label" == F10 || "$label" == Space || "$label" == Tab ]]; then
    if [[ "$reaches" != yes || "$act" == yes ]]; then
      echo "P1 RED: $BROWSER $label reachesPage=$reaches browserActed=$act; stopping experiments" >&2
      return 20
    fi
  fi
}
click_pad() { xdotool mousemove --window "$WIN" 500 400 click 1; sleep .8; }
if [[ "$MODE" == control ]]; then
  start_browser control-ctrl-t
  before=$(title); screen=$(snapshot)
  observe_key Ctrl+T ctrl+t "$before" "$screen"
  [[ "$(title)" == *"New Tab"* ]] || { echo "Ctrl+T new-tab title not observed" >&2; exit 11; }
  stop_browser
  start_browser control-ctrl-w
  before=$(title); screen=$(snapshot)
  xdotool key --clearmodifiers ctrl+w; sleep 1.2
  if xdotool search --onlyvisible --class "$CLASS" 2>/dev/null | rg -qx "$WIN"; then acted=no; evidence="visible browser window $WIN remains; title=$(title)"; else acted=yes; evidence="window $WIN no longer visible to xdotool; title=$(title)"; fi
  append "$BROWSER" control-ctrl-w Ctrl+W "$acted" "$evidence" "$RUN_ID"
  echo "control Ctrl+W browser acted=$acted; $evidence"
  [[ "$acted" == yes ]] || exit 11
  stop_browser
elif [[ "$MODE" == ctrl9 ]]; then
  start_browser ctrl9-retest
  for n in $(seq 2 10); do
    xdotool key --clearmodifiers ctrl+t; sleep .15
    xdotool type --clearmodifiers "http://127.0.0.1:8765/?browser=$BROWSER&group=ctrl9-retest&runId=$RUN_ID&tab=$n"; xdotool key Return; sleep .3
  done
  # Select the first tab by clicking it; Ctrl+9 should then select the distinct last tab if the browser acts.
  xdotool mousemove --window "$WIN" 70 30 click 1; sleep .5
  before=$(title); screen=$(snapshot); observe_key Ctrl+Digit9 ctrl+9 "$before" "$screen"
  stop_browser
elif [[ "$MODE" == pointers ]]; then
  start_browser pointer_inputs
  click_pad
  xdotool mousemove --window "$WIN" 350 350 click 1
  xdotool mousemove --window "$WIN" 350 350 mousedown 1
  xdotool mousemove --window "$WIN" 500 430
  xdotool mouseup 1
  xdotool mousemove --window "$WIN" 500 400 click 3; sleep .4
  xdotool mousemove --window "$WIN" 500 400 click 2; sleep .3
  xdotool mousemove --window "$WIN" 500 400 click 4; sleep .3
  xdotool keydown ctrl; xdotool mousemove --window "$WIN" 500 400 click 4; xdotool keyup ctrl; sleep .5
  xdotool mousemove --window "$WIN" 1 400; sleep .2
  append "$BROWSER" pointer_inputs "pointer group" no "xdotool left click/drag, right click/contextmenu, middle click, wheel, Ctrl+wheel, pointer move to edge; inspect page pointer/wheel records" "$RUN_ID"
  stop_browser
elif [[ "$MODE" == keys ]]; then
  # Ctrl+digit actions are observable only with known, nonselected sentinel tabs.
  for group in ctrl_digits shift_digits digits function special arrows locked_esc; do
    start_browser "$group"
    if [[ "$group" == ctrl_digits ]]; then
      for n in $(seq 2 10); do
        xdotool key --clearmodifiers ctrl+t; sleep .15
        xdotool type --clearmodifiers "http://127.0.0.1:8765/?browser=$BROWSER&group=ctrl_digits&runId=$RUN_ID&tab=$n"; xdotool key Return; sleep .3
      done
    elif [[ "$group" != locked_esc ]]; then click_pad; fi
    case "$group" in
      ctrl_digits) labels=(Ctrl+Digit1 Ctrl+Digit2 Ctrl+Digit3 Ctrl+Digit4 Ctrl+Digit5 Ctrl+Digit6 Ctrl+Digit7 Ctrl+Digit8 Ctrl+Digit9); seqs=(ctrl+1 ctrl+2 ctrl+3 ctrl+4 ctrl+5 ctrl+6 ctrl+7 ctrl+8 ctrl+9);;
      shift_digits) labels=(Shift+Digit1 Shift+Digit2 Shift+Digit3 Shift+Digit4 Shift+Digit5 Shift+Digit6 Shift+Digit7 Shift+Digit8 Shift+Digit9); seqs=(shift+1 shift+2 shift+3 shift+4 shift+5 shift+6 shift+7 shift+8 shift+9);;
      digits) labels=(Digit1 Digit2 Digit3 Digit4 Digit5 Digit6 Digit7 Digit8 Digit9); seqs=(1 2 3 4 5 6 7 8 9);;
      function) labels=(F1 F2 F3 F4 F5 F6 F7 F8 F9 F10 F11 F12); seqs=(F1 F2 F3 F4 F5 F6 F7 F8 F9 F10 F11 F12);;
      special) labels=(Space Tab Esc); seqs=(space Tab Escape);;
      arrows) labels=(ArrowUp ArrowDown ArrowLeft ArrowRight); seqs=(Up Down Left Right);;
      locked_esc) click_pad; sleep .6; before=$(title); screen=$(snapshot); observe_key Esc Escape "$before" "$screen"; stop_browser; continue;;
    esac
    for i in "${!labels[@]}"; do
      if [[ "$group" == ctrl_digits && "${labels[$i]}" == Ctrl+Digit9 ]]; then
        # Make Ctrl+9 observable by selecting tab 1 before asking it to select the last tab.
        xdotool mousemove --window "$WIN" 70 30 click 1; sleep .35
      fi
      before=$(title); screen=$(snapshot); observe_key "${labels[$i]}" "${seqs[$i]}" "$before" "$screen"; sleep .15
    done
    stop_browser
  done
else echo "unknown mode: $MODE" >&2; exit 2
fi
