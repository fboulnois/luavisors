use mlua::prelude::*;
use smol::stream::StreamExt;

use crate::{process, unix};

/// Return the current process identifier
async fn pid(_lua: Lua, _: ()) -> LuaResult<u32> {
    Ok(std::process::id())
}

/// Sleep the Lua runtime for `n` seconds
async fn sleep(_lua: Lua, n: u64) -> LuaResult<u64> {
    smol::Timer::after(std::time::Duration::from_secs(n)).await;
    Ok(n)
}

/// Asynchronously call a Lua function every `n` seconds
async fn every(lua: Lua, (n, func, args): (u64, LuaFunction, LuaMultiValue)) -> LuaResult<()> {
    let weak_lua = lua.weak();
    smol::spawn(async move {
        let mut timer = smol::Timer::interval(std::time::Duration::from_secs(n));
        while let Some(_instant) = timer.next().await {
            // stop task if the Lua instance has been destroyed
            let Some(_lua) = weak_lua.try_upgrade() else {
                break;
            };
            if let Err(err) = func.call_async::<()>(args.clone()).await {
                eprintln!("error in 'init.every' task: {}", err);
            }
        }
    })
    .detach();
    Ok(())
}

/// Send a signal to a process from Lua
async fn kill(_lua: Lua, (pid, sig): (i32, i32)) -> LuaResult<i32> {
    unix::kill(pid, sig)
        .await
        .map_err(|err| LuaError::runtime(err))
}

/// Return the `init` Lua module
pub async fn init(lua: Lua, _: ()) -> LuaResult<LuaTable> {
    let init = lua.create_table()?;
    init.set("exec", lua.create_async_function(process::exec)?)?;
    init.set("kill", lua.create_async_function(kill)?)?;
    init.set("pid", lua.create_async_function(pid)?)?;
    init.set("sleep", lua.create_async_function(sleep)?)?;
    init.set("every", lua.create_async_function(every)?)?;
    init.set("signal", lua.create_table_from(unix::signal_table())?)?;
    Ok(init)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_getpid() {
        let lua = Lua::new();
        let result = smol::block_on(pid(lua, ()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), std::process::id());
    }

    #[test]
    fn test_sleep() {
        let lua = Lua::new();
        let n = 0;
        let result = smol::block_on(sleep(lua, n));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), n);
    }

    #[test]
    fn test_every() {
        let lua = Lua::new();
        let n = 0;
        let func = lua.create_function(|_, ()| Ok(())).unwrap();
        let result = smol::block_on(every(lua, (n, func, LuaMultiValue::new())));
        assert!(result.is_ok());
    }

    #[test]
    fn test_every_with_error() {
        let lua = Lua::new();
        let globals = lua.globals();
        globals.set("count", 0).unwrap();
        let n = 0;
        let code = r#"
                count = count + 1
                if count == 1 then
                    error("Boom!")
                end
            "#;
        let func = lua.load(code).into_function().unwrap();
        smol::block_on(async {
            every(lua.clone(), (n, func, LuaMultiValue::new()))
                .await
                .unwrap();
            smol::Timer::after(std::time::Duration::from_millis(20)).await;
            let count: i32 = globals.get("count").unwrap();
            // ensure that function continues to run after error
            assert!(count > 1);
        });
    }

    #[test]
    fn test_kill() {
        let lua = Lua::new();
        let result = smol::block_on(kill(lua, (0, 0)));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_kill_err() {
        let lua = Lua::new();
        let result = smol::block_on(kill(lua, (0, 1337)));
        assert!(result.is_err());
    }

    #[test]
    fn test_init() {
        let lua = Lua::new();
        let result = smol::block_on(init(lua, ()));
        assert!(result.is_ok());
    }
}
