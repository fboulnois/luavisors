-- Example which launches a main process and an update process which stops the
-- main process and restarts the main process when the update process completes

-- load the `init` module
local init = require('init')

-- get name of current process from arg table
local exe = arg[-1]

-- simulate long running process
local function exec_process(sec)
    local args = string.format("local init = require('init'); init.sleep(%d)", sec)
    return init.exec(exe, args)
end

-- simulate executing software
local function exec_software()
    return exec_process(3)
end

-- simulate executing update
local function exec_update()
    return exec_process(1)
end

local software = exec_software()
print('software started with pid', software:pid())

-- simulate update check which stops the software, updates, and restarts it
local function check_for_updates()
    -- stop the existing software
    assert(software:kill() == init.signal.SIGKILL)
    -- check for updates
    print('checking for software updates')
    local update = exec_update()
    assert(update:status() == 0)
    print('completed software update')
    -- launch the "updated" software
    software = exec_software()
    print('software restarted with pid', software:pid())
    assert(software:status() == 0)
    print('software completed')
end

init.every(5, check_for_updates)

-- wait for software to finish
init.sleep(10)
