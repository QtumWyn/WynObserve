section .text

global wyn_rdtscp

wyn_rdtscp:
    rdtscp
    shl rdx, 32
    or rax, rdx
    ret