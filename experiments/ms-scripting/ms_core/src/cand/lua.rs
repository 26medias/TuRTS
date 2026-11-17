//! C3: Lua through piccolo 0.3.3 (pure Rust, stackless, fuel-driven).
//! Loaded libraries: base, coroutine, string, table. Not loaded: io (print, files) and math
//! (its `load_math` seeds `SmallRng::from_entropy`, an ambient entropy source).

use piccolo::{
    Callback, CallbackReturn, Closure, Context, Executor, Fuel, Function, Lua, StashedExecutor,
    StashedFunction, Table, Value,
};

use crate::cand::{self as c, Limits, Runtime, ScriptError, ScriptErrorKind};
use crate::host;

/// Fuel for one `step` between host checks.
const CHUNK: i32 = 4_096;

pub struct LuaRt {
    lua: Lua,
    exec: StashedExecutor,
    f_tick: Option<StashedFunction>,
    f_bench: Option<StashedFunction>,
    f_unit: Option<StashedFunction>,
    limits: Limits,
    name: String,
    last_fuel: u64,
}

fn line_from(msg: &str) -> Option<u32> {
    // piccolo formats compile errors with "line N" or ":N:"; take the first number after it.
    for key in ["line ", ":"] {
        if let Some(i) = msg.find(key) {
            let rest = &msg[i + key.len()..];
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = digits.parse() {
                return Some(n);
            }
        }
    }
    None
}

fn reg_i<'gc, const N: usize>(
    ctx: Context<'gc>,
    name: &'static str,
    f: impl Fn([i64; N]) -> Option<i64> + 'static,
) {
    let cb = Callback::from_fn(&ctx, move |ctx, _, mut stack| {
        let mut a = [0i64; N];
        for (i, slot) in a.iter_mut().enumerate() {
            *slot = match stack.get(i) {
                Value::Integer(v) => v,
                v => {
                    return Err(piccolo::Value::String(piccolo::String::from_slice(
                        &ctx,
                        format!("{name}: argument {} must be an integer, got {}", i + 1, v.type_name())
                            .as_bytes(),
                    ))
                    .into())
                }
            };
        }
        stack.clear();
        if let Some(r) = f(a) {
            stack.push_back(Value::Integer(r));
        }
        Ok(CallbackReturn::Return)
    });
    ctx.set_global(name, cb).unwrap();
}

impl LuaRt {
    pub fn new(name: &str, source: &str, limits: Limits) -> Result<Self, ScriptError> {
        let mut lua = Lua::empty();
        lua.enter(|ctx| {
            piccolo::stdlib::load_base(ctx);
            piccolo::stdlib::load_coroutine(ctx);
            piccolo::stdlib::load_string(ctx);
            piccolo::stdlib::load_table(ctx);

            reg_i::<2>(ctx, "count_owned", |a| Some(host::count_owned(a[0], a[1])));
            reg_i::<1>(ctx, "credits", |a| Some(host::credits(a[0])));
            reg_i::<2>(ctx, "grant", |a| {
                host::grant(a[0], a[1]);
                None
            });
            reg_i::<3>(ctx, "move_to", |a| {
                host::move_to(a[0], a[1], a[2]);
                None
            });
            reg_i::<2>(ctx, "set_stance", |a| {
                host::set_stance(a[0], a[1]);
                None
            });
            reg_i::<2>(ctx, "alert", |a| {
                host::alert(a[0], a[1]);
                None
            });
            let near = Callback::from_fn(&ctx, |ctx, _, mut stack| {
                let (p, x, y, r, m): (i64, i64, i64, i64, i64) = stack.consume(ctx)?;
                let t = Table::new(&ctx);
                for (i, id) in host::units_near(p, x, y, r, m).into_iter().enumerate() {
                    t.set(ctx, i as i64 + 1, id)?;
                }
                stack.push_back(Value::Table(t));
                Ok(CallbackReturn::Return)
            });
            ctx.set_global("units_near", near).unwrap();
            let group = Callback::from_fn(&ctx, |ctx, _, mut stack| {
                let (t, x, y): (Table, i64, i64) = stack.consume(ctx)?;
                let n = t.length();
                let mut v = Vec::with_capacity(n.max(0) as usize);
                for i in 1..=n {
                    v.push(match t.get(ctx, i) {
                        Value::Integer(id) => id,
                        _ => -1,
                    });
                }
                host::move_group(v, x, y);
                Ok(CallbackReturn::Return)
            });
            ctx.set_global("move_group", group).unwrap();
        });

        let chunk_name = name.to_string();
        let exec = lua
            .try_enter(|ctx| {
                let closure = Closure::load(ctx, Some(&chunk_name), source.as_bytes())?;
                Ok(ctx.stash(Executor::start(ctx, closure.into(), ())))
            })
            .map_err(|e| {
                let m = format!("{name}: {e}");
                ScriptError::new(ScriptErrorKind::Syntax, m.clone(), line_from(&m))
            })?;

        let fetch = |lua: &mut Lua, f: &'static str| -> Result<StashedFunction, ScriptError> {
            lua.try_enter(|ctx| {
                let v = ctx.get_global(f);
                match v {
                    Value::Function(func) => Ok(ctx.stash(func)),
                    _ => Err(piccolo::Value::String(piccolo::String::from_slice(
                        &ctx,
                        format!("function {f} is not defined").as_bytes(),
                    ))
                    .into()),
                }
            })
            .map_err(|e| ScriptError::new(ScriptErrorKind::Runtime, format!("{e}"), None))
        };

        let mut rt = LuaRt {
            lua,
            exec,
            f_tick: None,
            f_bench: None,
            f_unit: None,
            limits,
            name: name.to_string(),
            last_fuel: 0,
        };
        // Run the chunk body once (it defines the functions and the state), under the limits.
        rt.drive()?;
        // A missing entry function is an error only when the host calls it.
        rt.f_tick = fetch(&mut rt.lua, "on_tick").ok();
        rt.f_bench = fetch(&mut rt.lua, "bench_calls").ok();
        rt.f_unit = fetch(&mut rt.lua, "on_unit").ok();
        Ok(rt)
    }

    /// Step the executor under the fuel budget and the memory cap. Return its result.
    fn drive(&mut self) -> Result<Option<i64>, ScriptError> {
        let mut used: u64 = 0;
        loop {
            let mut fuel = Fuel::with(CHUNK);
            let done = self
                .lua
                .enter(|ctx| ctx.fetch(&self.exec).step(ctx, &mut fuel));
            used += (i64::from(CHUNK) - i64::from(fuel.remaining())).max(0) as u64;
            if done {
                break;
            }
            if used >= self.limits.budget {
                self.last_fuel = used;
                return Err(ScriptError::new(
                    ScriptErrorKind::Budget,
                    format!("{}: fuel budget of {} exhausted", self.name, self.limits.budget),
                    None,
                ));
            }
            if c::call_over_cap() && {
                self.lua.gc_collect();
                c::call_over_cap()
            } {
                self.last_fuel = used;
                return Err(ScriptError::new(
                    ScriptErrorKind::Memory,
                    format!(
                        "{}: script memory grew over the room of {} bytes",
                        self.name,
                        c::call_room()
                    ),
                    None,
                ));
            }
        }
        self.last_fuel = used;
        if c::call_over_cap() && {
            self.lua.gc_collect();
            c::call_over_cap()
        } {
            return Err(ScriptError::new(
                ScriptErrorKind::Memory,
                format!(
                    "{}: script memory grew over the room of {} bytes",
                    self.name,
                    c::call_room()
                ),
                None,
            ));
        }
        let name = self.name.clone();
        self.lua
            .try_enter(|ctx| {
                let r = ctx.fetch(&self.exec).take_result::<Value>(ctx)?;
                Ok(r.map(|v| match v {
                    Value::Integer(i) => Some(i),
                    _ => None,
                })
                .map_err(|e| e.into_static()))
            })
            .map_err(|e| ScriptError::new(ScriptErrorKind::Runtime, format!("{name}: {e}"), None))?
            .map_err(|e| {
                let m = format!("{name}: {e}");
                ScriptError::new(ScriptErrorKind::Runtime, m.clone(), line_from(&m))
            })
    }

    fn call(
        &mut self,
        which: u8,
        args: impl for<'gc> FnOnce(Context<'gc>) -> Vec<Value<'gc>>,
    ) -> Result<Option<i64>, ScriptError> {
        let f = match which {
            0 => self.f_tick.as_ref(),
            1 => self.f_bench.as_ref(),
            _ => self.f_unit.as_ref(),
        };
        let Some(f) = f else {
            return Err(ScriptError::new(
                ScriptErrorKind::Runtime,
                format!("{}: entry function {} is not defined", self.name, ["on_tick", "bench_calls", "on_unit"][which.min(2) as usize]),
                None,
            ));
        };
        self.lua.enter(|ctx| {
            let func: Function = ctx.fetch(f);
            let a = args(ctx);
            ctx.fetch(&self.exec).restart(ctx, func, piccolo::Variadic(a));
        });
        self.drive()
    }
}

impl Runtime for LuaRt {
    fn on_tick(&mut self, events: &[i64]) -> Result<(), ScriptError> {
        self.call(0, |ctx| {
            let t = Table::new(&ctx);
            for (i, v) in events.iter().enumerate() {
                t.set(ctx, i as i64 + 1, *v).unwrap();
            }
            vec![Value::Table(t)]
        })
        .map(|_| ())
    }

    fn bench_calls(&mut self, n: i64) -> Result<i64, ScriptError> {
        Ok(self.call(1, |_| vec![Value::Integer(n)])?.unwrap_or(-1))
    }

    fn on_unit(&mut self, id: i64, x: i64, y: i64, owner: i64) -> Result<(), ScriptError> {
        self.call(2, |_| {
            vec![
                Value::Integer(id),
                Value::Integer(x),
                Value::Integer(y),
                Value::Integer(owner),
            ]
        })
        .map(|_| ())
    }

    fn last_units(&mut self) -> Option<u64> {
        Some(self.last_fuel)
    }

    fn collect(&mut self) {
        self.lua.gc_collect();
    }
}
