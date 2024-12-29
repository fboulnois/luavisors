-- Example which shows the usage of the `init` API

-- Load the API
local init = require('init')

-- Print parent process id
print('parent pid:', init.pid())

-- Start a child process asynchronously
local sleep1 = init.exec('sleep', '5')

-- Print child process id
print('sleep pid:', sleep1:pid())

-- Start another child process asynchronously
local echo = init.exec('echo', 'hello', 'world')

-- Print child process output, will print before sleep finishes
print('echo stdout:', echo:stdout())

-- Check that no error occurred
print('echo no error:', echo:stderr() == nil)

-- Terminate the child process directly
sleep1:kill()

-- Verify that the sleep process was killed
print('sleep1 exited with SIGKILL:', sleep1:status() == init.signal.SIGKILL)

-- Start another sleep process asynchronously
local sleep2 = init.exec('sleep', '5')

-- Send a signal to the child process
init.kill(sleep2:pid(), init.signal.SIGTERM)

-- Verify that the sleep process was killed
print('sleep2 exited with SIGTERM:', sleep2:status() == init.signal.SIGTERM)

-- Start a third sleep process asynchronously
local sleep3 = init.exec('sleep', '1')

-- Verify that the sleep process exited normally
print('sleep3 exited normally:', sleep3:status() == 0)

-- Start an echo process asynchronously
local function echo(...)
    local child = init.exec('echo', ...)
    print('echo stdout:', child:stdout())
end

-- Run the echo function every 2 seconds
init.every(2, echo, 'hello', 'world')

-- Sleep for 5 seconds to allow the echo function to run
init.sleep(5)
