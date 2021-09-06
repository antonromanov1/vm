.global main

.text

main:
    mov $1, %rax
    mov $0, %rbx
    mov $1, %rdx
    mov $6, %rcx

L1:
    mov %rbx, %rax
    add %rdx, %rax
    mov %rdx, %rbx
    mov %rax, %rdx
    loop L1
    ret
