use std::{ffi::OsStr, os::unix::process::ExitStatusExt, sync::Arc};

use async_signal::Signal;
use mlua::prelude::*;
use smol::{
    io::AsyncReadExt,
    lock::{Mutex, RwLock},
    process::{Child, Stdio},
    stream::StreamExt,
};

use crate::{errors::AppResult, unix};

/// Forward signals to the child process
async fn forward_signals(child: Arc<RwLock<Child>>) -> AppResult<()> {
    let pid = child.read().await.id() as i32;
    let mut signals = unix::signal_wait().await?;
    while let Some(signal) = signals.next().await {
        let sig = signal? as i32;
        unix::kill(pid, sig).await?;
    }
    Ok(())
}

/// Spawn a new process asynchronously
async fn spawn<S, I>(program: S, args: I) -> std::io::Result<Child>
where
    S: AsRef<OsStr>,
    I: IntoIterator<Item = S>,
{
    let mut cmd = smol::process::Command::new(&program);
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
    cmd.spawn()
}

/// Spawn a new process from Lua
async fn lua_spawn(_lua: &Lua, cmd: String, args: LuaMultiValue) -> LuaResult<Child> {
    let mut vargs = Vec::new();
    for arg in args {
        match arg {
            LuaValue::Table(t) => vargs.extend(
                t.sequence_values::<String>()
                    .collect::<LuaResult<Vec<_>>>()?,
            ),
            _ => vargs.push(arg.to_string()?),
        }
    }
    Ok(spawn(cmd, vargs).await?)
}

/// Spawn a task to read from a stream
async fn spawn_stream_task(
    stream: Option<impl AsyncReadExt + Unpin + Send + 'static>,
) -> Arc<Mutex<Option<smol::Task<std::io::Result<Vec<u8>>>>>> {
    let task = stream.map(|mut stream| {
        smol::spawn(async move {
            let mut data = Vec::new();
            stream.read_to_end(&mut data).await?;
            Ok(data)
        })
    });
    Arc::new(Mutex::new(task))
}

/// Read a stream into a Lua string
async fn read_stream_task(
    lua: Lua,
    task: Arc<Mutex<Option<smol::Task<std::io::Result<Vec<u8>>>>>>,
) -> LuaResult<LuaValue> {
    let task = task.lock().await.take().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "stream already consumed")
    })?;
    let data = task.await?;
    if data.is_empty() {
        return Ok(LuaValue::Nil);
    }
    Ok(LuaValue::String(lua.create_string(&data)?))
}

/// Asynchronously execute a command in Lua
pub async fn exec(lua: Lua, (cmd, args): (String, LuaMultiValue)) -> LuaResult<LuaTable> {
    let mut child = lua_spawn(&lua, cmd, args).await?;

    let stdout = spawn_stream_task(child.stdout.take()).await;
    let stderr = spawn_stream_task(child.stderr.take()).await;

    let child = Arc::new(RwLock::new(child));

    smol::spawn(forward_signals(child.clone())).detach();

    let result = lua.create_table()?;

    // pid
    let clone = child.clone();
    result.set(
        "pid",
        lua.create_async_function(move |_, ()| {
            let child = clone.clone();
            async move { Ok(child.read().await.id()) }
        })?,
    )?;

    // status
    let clone = child.clone();
    result.set(
        "status",
        lua.create_async_function(move |_, ()| {
            let child = clone.clone();
            async move {
                let status = child.write().await.status().await?;
                let code = status
                    .signal()
                    .or_else(|| status.code())
                    .ok_or(LuaError::runtime("failed to get status code"))?;
                Ok(code)
            }
        })?,
    )?;

    // stdout
    result.set(
        "stdout",
        lua.create_async_function(move |lua, ()| {
            let task = stdout.clone();
            async move { read_stream_task(lua, task).await }
        })?,
    )?;

    // stderr
    result.set(
        "stderr",
        lua.create_async_function(move |lua, ()| {
            let task = stderr.clone();
            async move { read_stream_task(lua, task).await }
        })?,
    )?;

    // kill
    let clone = child.clone();
    result.set(
        "kill",
        lua.create_async_function(move |_, ()| {
            let child = clone.clone();
            async move {
                child.write().await.kill()?;
                Ok(Signal::Kill as i32)
            }
        })?,
    )?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_setup_spawn() -> std::io::Result<Child> {
        spawn("rustc", ["--version"]).await
    }

    async fn test_setup_exec(lua: &Lua) -> LuaResult<LuaTable> {
        let cmd = "rustc".to_string();
        let args = LuaMultiValue::new();
        exec(lua.clone(), (cmd, args)).await
    }

    #[test]
    fn test_spawn() {
        smol::block_on(async {
            let mut child = test_setup_spawn().await.unwrap();
            let status = child.status().await.unwrap();
            assert!(status.success());
        });
    }

    #[test]
    fn test_lua_spawn() {
        smol::block_on(async {
            let lua = Lua::new();
            let cmd = "rustc".to_string();
            let args = LuaMultiValue::from(vec![LuaValue::String(
                lua.create_string("--version").unwrap(),
            )]);
            let mut child = lua_spawn(&lua, cmd, args).await.unwrap();
            let status = child.status().await.unwrap();
            assert!(status.success());
        });
    }

    #[test]
    fn test_lua_spawn_with_table() {
        smol::block_on(async {
            let lua = Lua::new();
            let cmd = "rustc".to_string();
            let table = lua.create_table().unwrap();
            table.set(1, "--version").unwrap();
            let args = LuaMultiValue::from(vec![LuaValue::Table(table)]);
            let mut child = lua_spawn(&lua, cmd, args).await.unwrap();
            let status = child.status().await.unwrap();
            assert!(status.success());
        });
    }

    #[test]
    fn test_spawn_stream_task_stdout() {
        smol::block_on(async {
            let mut child = test_setup_spawn().await.unwrap();
            let task = spawn_stream_task(child.stdout.take()).await;
            let data = task.lock().await.take().unwrap().await.unwrap();
            assert!(data.starts_with(b"rustc"));
        });
    }

    #[test]
    fn test_spawn_stream_task_stderr() {
        smol::block_on(async {
            let mut child = test_setup_spawn().await.unwrap();
            let task = spawn_stream_task(child.stderr.take()).await;
            let data = task.lock().await.take().unwrap().await.unwrap();
            assert!(data.is_empty());
        });
    }

    #[test]
    fn test_spawn_stream_task_none() {
        smol::block_on(async {
            let lua = Lua::new();
            let task = spawn_stream_task(None::<smol::io::Empty>).await;
            let result = read_stream_task(lua, task).await;
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_read_stream_task() {
        smol::block_on(async {
            let lua = Lua::new();
            let mut child = test_setup_spawn().await.unwrap();
            let task = spawn_stream_task(child.stdout.take()).await;
            let value = read_stream_task(lua.clone(), task).await.unwrap();
            assert!(matches!(value, LuaValue::String(_)));
            assert!(value.to_string().unwrap().starts_with("rustc"));
        });
    }

    #[test]
    fn test_read_stream_task_empty() {
        smol::block_on(async {
            let lua = Lua::new();
            let mut child = test_setup_spawn().await.unwrap();
            let task = spawn_stream_task(child.stderr.take()).await;
            let value = read_stream_task(lua.clone(), task).await.unwrap();
            assert!(matches!(value, LuaValue::Nil));
        });
    }

    #[test]
    fn test_read_stream_none() {
        smol::block_on(async {
            let lua = Lua::new();
            let none = Arc::new(Mutex::new(None));
            let result = read_stream_task(lua.clone(), none).await;
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_exec() {
        smol::block_on(async {
            let lua = Lua::new();
            assert!(test_setup_exec(&lua).await.is_ok());
        });
    }

    #[test]
    fn test_exec_pid() {
        smol::block_on(async {
            let lua = Lua::new();
            let table = test_setup_exec(&lua).await.unwrap();
            let pid = table.get::<LuaFunction>("pid").unwrap();
            assert!(pid.call_async::<i32>(()).await.is_ok());
        });
    }

    #[test]
    fn test_exec_status() {
        smol::block_on(async {
            let lua = Lua::new();
            let table = test_setup_exec(&lua).await.unwrap();
            let status = table.get::<LuaFunction>("status").unwrap();
            assert!(status.call_async::<i32>(()).await.is_ok());
        });
    }

    #[test]
    fn test_exec_stdout() {
        smol::block_on(async {
            let lua = Lua::new();
            let table = test_setup_exec(&lua).await.unwrap();
            let stdout = table.get::<LuaFunction>("stdout").unwrap();
            assert!(stdout
                .call_async::<Option<String>>(())
                .await
                .unwrap()
                .is_some());
        });
    }

    #[test]
    fn test_exec_stderr() {
        smol::block_on(async {
            let lua = Lua::new();
            let table = test_setup_exec(&lua).await.unwrap();
            let stderr = table.get::<LuaFunction>("stderr").unwrap();
            // stderr is empty and returns nil
            assert!(stderr
                .call_async::<Option<String>>(())
                .await
                .unwrap()
                .is_none());
        });
    }

    #[test]
    fn test_exec_kill() {
        smol::block_on(async {
            let lua = Lua::new();
            let table = test_setup_exec(&lua).await.unwrap();
            let kill = table.get::<LuaFunction>("kill").unwrap();
            assert!(kill.call_async::<i32>(()).await.is_ok());
        });
    }
}
