# 声笔输入法自动更新程序

自动检测 gitee.com 上最新的 release 信息，如果本地安装的和 release 的不一致，则进行更新。
支持更新前备份原有配置。

## 快速开始

```shell
./sbsrf-update
```

## 参数说明：

```text
Usage: sbsrf-update [OPTIONS]

Options:
  -f, --force                      强制更新，默认本地版本和服务器版本一致时不作任何操作，强制更新时即使版本相同也会更新
  -p, --platform <PLATFORM>        目标操作系统，默认为当前系统，用于匹配需要下载的文件
  -w, --working-dir <WORKING_DIR>  工作目录，默认在 $HOME/.sbsrf-update
  -h, --help                       打印帮助信息
  -V, --version                    打印本程序的版本信息
```
