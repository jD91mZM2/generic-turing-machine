// In binary, something being divisible by 3 means all numbers at the odd
// positions minus all numbes at the even positions is also divisible by 3.
// Compare this to the trick to finding 11 in decimal, to see how it works.

// Even better, in binary the only way to actually unbalance the numbers is
// typing 10, and then that can be reset with 01.
// In fact, you can even match the whole thing with a regex:
// (0|11|10(1|00)*01)*

start = even

even 0 = 0; even next
even 1 = 1; switch next
even _ = _; finish current

switch 0 = 0; odd next
switch 1 = 1; even next

odd 1 = 1; odd next
odd 0 = 0; switch next
