# In regards to compatibility

The authors have made a best effort attempt to replicate the behavior of `Ink` in 
most respects. However, since this is a completely separate implementation, there 
are surely certain features which even if implemented 
([and many aren't](./../features/missing-features.md)), result in different output 
from the same input `.ink` script. Edge cases may be a-plenty. 

We do care about making `Ink` and `inkling` as similar as possible, to the extent 
that we can. If you find cases where the results differ, please let us know by 
opening an issue on the [Github repository](https://github.com/pjohansson/inkling).

In the end, `inkling` cannot be considered a drop-in replacement for `Inkle's` own 
implementation of the language. More realistically, it's inspired by it, sharing 
most features but with results which may differ. Keep this in mind while writing 
a script.

 — Petter Johansson, 2020
