```bash
task run L=256 M=256 N=256 MODE=all REPS=1

task run-novec L=128 M=128 N=128 MODE=all
```


```bash
task build
samply record ./target/release/lab2_novec --mode scalar --l 256 --m 256 --n 256 --reps 2
samply record ./target/release/lab2_vec   --mode neon   --l 256 --m 256 --n 256 --reps 2
samply record ./target/release/lab2_vec   --mode l3     --l 256 --m 256 --n 256 --reps 2
```

```bash
cargo install cargo-show-asm

task asm-neon    
task asm-l3     
task asm       
```
