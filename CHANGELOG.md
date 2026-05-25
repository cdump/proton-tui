# Changelog

## [0.3.0](https://github.com/cdump/proton-tui/compare/v0.2.0...v0.3.0) (2026-05-25)


### Features

* add configuration system and persist split view and group by ip preferences ([7771645](https://github.com/cdump/proton-tui/commit/7771645d006b17acedc3e578dfef74123323eb1a))


### Bug Fixes

* **auth:** keep refreshed client after relogin ([f16967a](https://github.com/cdump/proton-tui/commit/f16967a527c190498d9a5d5bf9960c56f450438a))
* **connection:** avoid deleting scoped vpn certificates ([97afe06](https://github.com/cdump/proton-tui/commit/97afe060eca548da8381ebecf31999da8ba02c40))


### Performance Improvements

* **app:** optimize search by pre-computing lowercase strings ([1eda8da](https://github.com/cdump/proton-tui/commit/1eda8daee4d795b55653a3a9cb4f978fa5842ef5))


### Miscellaneous Chores

* **release:** force release-please update ([fd2d7ce](https://github.com/cdump/proton-tui/commit/fd2d7ce503f8902e3f73959b63da9912627f16f0))

## [0.2.0](https://github.com/cdump/proton-tui/compare/v0.1.0...v0.2.0) (2026-01-14)


### Features

* add grouping by exit IP and refine help UI ([b04cfb5](https://github.com/cdump/proton-tui/commit/b04cfb5d8da90762f37644e7166d80897a97726e))
* **config:** use XDG dirs for wg configs ([57edf6b](https://github.com/cdump/proton-tui/commit/57edf6b39429b7ef69deadba487bd0ae1f5e59c5))
* group servers by entry_ip instead of exit_ip ([763b053](https://github.com/cdump/proton-tui/commit/763b0535630ee7b956995a68033d2560173b166b))
* **ui:** add vim Ctrl-u/Ctrl-d paging ([edc33ee](https://github.com/cdump/proton-tui/commit/edc33ee71d8b878019f8b6eee1c14410d07c0850))
* **ui:** display country and server counts in split view headers ([f155926](https://github.com/cdump/proton-tui/commit/f15592620f927bf735269dee9eed1b98c79227b8))


### Bug Fixes

* **search:** include country names in tree view filtering ([8beacab](https://github.com/cdump/proton-tui/commit/8beacab6778fed3c439f5bcc2d28f5b47e56a9ea))
* **ui:** remove redundant Exit IPs from status bar ([8c0fc5a](https://github.com/cdump/proton-tui/commit/8c0fc5af0cf95ade5aba8fac644d93a1a1df4689))

## 0.1.0 (2026-01-13)


### Miscellaneous Chores

* force release 0.1.0 ([2dafca8](https://github.com/cdump/proton-tui/commit/2dafca89edb6f46e371bfc43b82d2239cbf63775))
