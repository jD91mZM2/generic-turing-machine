start = init

init 0 = _; back<inc> next
init 1 = _; back<dec> next

back<fn> 0 = 0; back<fn> next
back<fn> 1 = 1; back<fn> next
back<fn> _ = _; fn prev

inc 1 = 0; inc prev
inc 0 = 1; finish current
inc _ = 1; finish current

dec 0 = 1; dec prev
dec 1 = 0; finish current
