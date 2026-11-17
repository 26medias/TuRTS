-- MS mod rules (spec 3.2), Lua (piccolo). State is local to the chunk.
-- Event record: [kind, player, unit_type, unit, extra, x, y]. 1 Died, 2 Produced,
-- 3 Researched, 4 Timer. Players and types are 0-based; Lua tables use index + 1.

local kills = {0, 0, 0, 0, 0, 0, 0, 0}
local rx, ry = {}, {}
for p = 0, 7 do
  rx[p + 1] = 16000 + p * 28000
  ry[p + 1] = 128000
end
local leader = -1
local counter = 0
local bounty = {50, 75, 150, 400}

function on_tick(ev)
  local n = #ev
  local i = 1
  while i <= n do
    local kind = ev[i]
    local player = ev[i + 1]
    if kind == 1 then
      local killer = ev[i + 4]
      local k = killer + 1
      kills[k] = kills[k] + 1
      local b = bounty[ev[i + 2] + 1]
      if kills[k] % 10 == 0 then
        b = b * 2
      end
      grant(killer, b)
      local near = units_near(player, ev[i + 5], ev[i + 6], 8000, 16)
      if #near < 3 then
        alert(player, 1)
      else
        move_group(near, rx[player + 1], ry[player + 1])
      end
    elseif kind == 2 then
      local c = count_owned(player, ev[i + 2])
      if c > 30 then
        set_stance(ev[i + 3], 2)
      else
        move_to(ev[i + 3], rx[player + 1], ry[player + 1])
      end
    elseif kind == 3 then
      local p = player + 1
      rx[p] = rx[p] + 1000 * (ev[i + 4] % 3 + 1)
      ry[p] = ry[p] - 500
      if rx[p] > 250000 then
        rx[p] = 6000
      end
      if ry[p] < 6000 then
        ry[p] = 250000
      end
    elseif kind == 4 then
      local best, best_c = -1, -1
      for p = 0, 7 do
        local c = credits(p)
        if c > best_c then
          best = p
          best_c = c
        end
      end
      if best ~= leader then
        leader = best
        alert(best, 2)
      end
    end
    i = i + 7
  end
end

function bench_calls(n)
  local s = 0
  local i = 0
  while i < n do
    s = s + count_owned(i % 8, i % 4)
    i = i + 1
  end
  return s
end

function on_unit(id, x, y, owner)
  if x > y then
    counter = counter + 1
  end
end
