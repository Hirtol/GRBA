# Resource Links

## CPU
* [ARM Reference Manual](arm.pdf)
* [ARMv7 Datasheet](ARM7TDMI_Recent.pdf)
* [ARMv7 Instructions](arm-instructionset.pdf) (PSR instructions are wrong, see [ARM Reference Manual](arm.pdf))
* [ARMv7(Thumb) Datasheet](ARM7TDMI_Datasheet.pdf)
* [ARMv7 Timings](ARM7TDMI_Instruction_Timings.pdf)

## EEPROM
* [Dennis](https://densinh.github.io/DenSinH/emulation/2021/02/01/gba-eeprom.html)
* [GBA Save Systems Explained](https://dillonbeliveau.com/2020/06/05/GBA-FLASH.html)

## General
* [GBATek](https://problemkaputt.de/gbatek.htm)
* [Audio](http://belogic.com/gba/)
* [TONC](https://www.coranac.com/projects/tonc/) (GBA Tutorials and Demos)
* [MGBA Blog](https://mgba.io/2015/06/27/cycle-counting-prefetch/)
* [Homebrew Development](https://patater.com/gbaguy/gbaasm.htm)

## Demo Links
* [GBADev](https://www.gbadev.org/demos.php?showinfo=527)

### Secret Notes:

```
Does arm.gba test all the ldm/stm edge cases?

Not all of them, but most of them
It misses one that modern gcc actually uses

Which one?

Uhh I think thumb mode ldmia with rb in rlist
That one will destroy you if you try to add NDS support to your GBA emulator

Yeah, that's architecturally defined
Make sure not to miss it

Is it defined by ARM7TDMI or ARMv4T?
It's defined in the ARMv4 spec
```

### TODO:

* GBA System Control (HALTing, Waitstate, etc)