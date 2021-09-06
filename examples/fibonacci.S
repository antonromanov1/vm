ldai 1
print
movi v0, 0
movi v1, 0
movi v3, 1
movi v2, 6

L1:
lda v1
add v3
print
mov v1, v3
sta v3
dec v2
bne v2, v0, L1
