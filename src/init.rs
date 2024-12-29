use mlua::prelude::*;
use smol::stream::StreamExt;

use crate::{process, unix};

async fn pid(_lua: Lua, _: ()) -> LuaResult<u32> {
    Ok(std::process::id())
}

async fn sleep(_lua: Lua, n: u64) -> LuaResult<u64> {
    smol::Timer::after(std::time::Duration::from_secs(n)).await;
    Ok(n)
}

async fn every(_lua: Lua, (n, func, args): (u64, LuaFunction, LuaMultiValue)) -> LuaResult<()> {
    smol::spawn(async move {
        let mut timer = smol::Timer::interval(std::time::Duration::from_secs(n));
        while let Some(_instant) = timer.next().await {
            func.call_async::<()>(args.clone()).await?;
        }
        Ok::<_, LuaError>(())
    })
    .detach();
    Ok(())
}

async fn kill(_lua: Lua, (pid, sig): (i32, i32)) -> LuaResult<i32> {
    unix::kill(pid, sig)
        .await
        .map_err(|err| LuaError::runtime(err.to_string()))
}

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
