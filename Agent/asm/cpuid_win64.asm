section .text

global wyn_cpuid

wyn_cpuid:
    ; Windows x64: ECX=leaf, EDX=subleaf, R8=output.
    ; RBX is nonvolatile. Save it in volatile R9 to remain a stackless leaf
    ; (no unwind metadata or shadow-space allocation is needed).
    mov r9, rbx
    mov eax, ecx
    mov ecx, edx

    cpuid

    mov [r8], eax
    mov [r8 + 4], ebx
    mov [r8 + 8], ecx
    mov [r8 + 12], edx

    mov rbx, r9
    ret
