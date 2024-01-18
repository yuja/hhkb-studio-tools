Linux tool to modify HHKB Studio keymap
=======================================

Usage
-----

1. Find `/dev/hidraw*` device to communicate and adjust permission

```shell
$ sudo setfacl -m u:$USER:rw /dev/hidraw1
```

2. Query the keyboard to see if the communication channel works

```shell
$ hhkb-studio-tools info
Product name: HHKB-Studio
...
```

3. Fetch the current keymap data (of the current profile)

```shell
$ hhkb-studio-tools read-profile > profile.bin
```

4. Show the fetched keymap data and modify it by using binary editor

```shell
$ hhkb-studio-tools show-profile < profile.bin
```

- The keymap data consists of four layers (Base, Fn1, Fn2, and Fn3.)
- Each layer is 240 bytes (15 keys x 8 rows with some blank entries.)

5. Load the modified keymap data to the keyboard

```shell
$ hhkb-studio-tools write-profile < profile_new.bin
```
