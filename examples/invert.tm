start = invert

invert 0 = 1; invert next
invert 1 = 0; invert next
invert _ = _; finish current
