-- Example which shows the usage of the `init` API

-- Load the API
local init = require('init')

-- Print parent process id
print('parent pid:', init.pid())

-- Start child process asynchronously
local child1 = init.exec('printf', '%s', 'hello world')

-- print child process id
print('child1 pid:', child1:pid())

-- Print child process output
print('child1 stdout:', child1:stdout())

-- Check that no error occurred
print('child1 stderr is empty:', child1:stderr() == nil)

-- Check that process exited with code 0
print('child1 exited normally:', child1:status() == 0)

-- Start a child process asynchronously
local child2 = init.exec('sleep', 2)

-- Terminate the child process directly
child2:kill()

-- Verify that the child process was killed
print('child2 exited with SIGKILL:', child2:status() == init.signal.SIGKILL)

-- Start another child process asynchronously
local child3 = init.exec('sleep', 2)

-- Send a signal to the child process
init.kill(child3:pid(), init.signal.SIGTERM)

-- Verify that the child process was killed
print('child3 exited with SIGTERM:', child3:status() == init.signal.SIGTERM)

-- Define an asynchronous job that prints the current time and arguments
local function job(...)
    local child = init.exec('printf', '%s %s %s', os.date("%X"), ...)
    print('job stdout:', child:stdout())
end

-- Run the job function every second
init.every(1, job, 'hello', 'world')

-- Sleep for enough time to allow the job function to run multiple times
init.sleep(3)
