section .text

global wyn_cpuid

wyn_cpuid:
    push rbx
    mov eax, ecx
    mov ecx, edx

    cpuid

    mov [r8], eax
    mov [r8 + 4], ebx
    mov [r8 + 8], ecx
    mov [r8 + 12], edx

    pop rbx
    ret