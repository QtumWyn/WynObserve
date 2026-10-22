section .text

global wyn_rdtscp

wyn_rdtscp:
    ; RDTSCP orders prior execution; LFENCE also orders subsequent execution.
    rdtscp
    lfence
    shl rdx, 32
    or rax, rdx
    ret
