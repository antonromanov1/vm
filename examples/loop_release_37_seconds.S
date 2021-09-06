movi v0, 0
movi v2, 91615

L2:
dec v2
movi v1, 91615
L1:
dec v1
bne v0, v1, L1
bne v0, v2, L2
