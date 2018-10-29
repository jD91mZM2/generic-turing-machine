start = skip<skip<ifZero<finish, clear>>>

skip<fn> 0 = 0; fn next
skip<fn> 1 = 1; fn next
skip<fn> _ = _; fn next

ifZero<success, fail> 0 = 0; success current
ifZero<success, fail> 1 = 1; fail current
ifZero<success, fail> _ = _; fail current

clear 0 = _; finish current
clear 1 = _; finish current
clear _ = _; finish current
