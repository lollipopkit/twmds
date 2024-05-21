## twmds
批量下载 Twitter 媒体文件

### 用法
首先，需要安装 [twmd](https://github.com/mmpx12/twitter-media-downloader)

然后，进入想要保存媒体文件的文件夹，在该文件新建文件夹，名称是你想要下载的 Twitter 用户的 ID，类似：
```shell
.
├── 用户ID1
├── 用户ID2
└── 用户ID3
```

最后，在该文件夹下运行 `twmds` 命令：
```shell
twmds
# 可以选择不登录，登录需要先执行 `twmd -L`
twmds -n
```

### CLI
```shell
Usage: twmds [OPTIONS]

Options:
  -n, --no-login               Do not login
  -s, --sleep <SLEEP>          Sleep seconds [default: 15]
  -d, --skip-dirs <SKIP_DIRS>  Skip dirs
  -h, --help                   Print help
  -V, --version                Print version
```