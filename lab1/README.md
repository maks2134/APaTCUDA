# fast command
## - профайлинг:
```
task build-both

samply record ./target/release/lab1_novec --mode auto --l 64 --m 64 --n 64
samply record ./target/release/lab1_vec   --mode auto --l 64 --m 64 --n 64
samply record ./target/release/lab1_vec   --mode neon --l 64 --m 64 --n 64
```
## - дизасик:
```
task asm

task asm-novec

task asm-novec
```

## - запуск:
```
task run-vec L=64 M=64 N=64
task run-novec L=64 M=64 N=64
```