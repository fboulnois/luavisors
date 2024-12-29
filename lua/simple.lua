-- Example which launches two child processes and waits for them to finish

-- Load the API
local init = require('init')

-- Start two child processes asynchronously
local child1 = init.exec("sleep", 1)
local child2 = init.exec("echo", "hello")

-- Wait for the child processes to finish
assert(child1:status() == 0)
assert(child2:status() == 0)
