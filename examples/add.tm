// Convenience functions

r<f> 0 = 0; f next
r<f> 1 = 1; f next
r<f> _ = _; f next
right<f> 0 = 0; r<r<r<f>>> next
right<f> 1 = 1; r<r<r<f>>> next
right<f> _ = _; r<r<r<f>>> next

l<f> 0 = 0; f prev
l<f> 1 = 1; f prev
l<f> _ = _; f prev
left<f> 0 = 0; l<l<l<f>>> prev
left<f> 1 = 1; l<l<l<f>>> prev
left<f> _ = _; right<finish> current

// Ready, let's go

startAdd 0 = _; right<endAdd0> current
startAdd 1 = _; right<endAdd1> current

endAdd0 0 = _; right<put0> current
endAdd0 1 = _; right<put1> current
endAdd1 0 = _; right<put1> current
endAdd1 1 = _; right<put10> current

put0 0 = 0; left<left<startAdd>> prev
put0 1 = 1; left<left<startAdd>> prev
put1 0 = 1; left<left<startAdd>> prev
put1 1 = 0; put10 current

put10 0 = 0; endPut10 prev
put10 1 = 1; endPut10 prev

endPut10 0 = 1; left<left<startAdd>> current

start = r<r<r<startAdd>>>
