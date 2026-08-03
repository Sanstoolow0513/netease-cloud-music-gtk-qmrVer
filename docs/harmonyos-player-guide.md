# 网易云音乐鸿蒙（HarmonyOS）播放器实现指南

> **目标读者**：负责实现鸿蒙 App 的 AI 编码代理。读完本文档即可开工，无需再读桌面版源码。
>
> **事实来源**：桌面版 netease-cloud-music-gtk4 v2.5.3（GTK4/Rust）及其依赖 crate `ncm-api`（netease-cloud-music-api **tag 2.0.0**，commit `c25319c`）。本文档逐条摘自源码，凡未标注"可选/建议"的常量与参数都必须原样实现，不要"修正"任何看似笔误的字符串（见附录 B 已知怪癖）。
>
> **目标**：在 HarmonyOS（ArkTS/ArkUI）上实现与桌面版功能对齐的网易云音乐播放器：扫码/验证码登录、发现页、榜单、歌单/专辑详情、搜索、"我的"页、在线播放（含音质选择、缓存）、歌词。

---

## 1. 总体架构

桌面版没有自建后端，全部数据来自网易官方接口（`music.163.com` / `interface.music.163.com`），关键是**复刻其 HTTP + 加密层**。鸿蒙版建议分层：

```
┌─ UI 层（ArkUI 页面：发现/榜单/歌单详情/搜索/我的/播放栏/歌词）
├─ 业务层（页面数据加载、播放队列、收藏状态、登录状态机）
├─ API 层（NcmApi：每个接口一个方法，返回强类型模型）
├─ 网络层（NcmHttp：统一 POST 表单、请求头、Cookie 管理、UA 池、代理）
├─ 加密层（NcmCrypto：weapi / eapi 加密，AES/RSA/MD5）
├─ 持久层（Cookie 持久化、歌曲缓存、歌词缓存、图片缓存、设置项）
└─ 播放器层（AVPlayer 封装：进度、切歌、缓存落盘、AVSession 后台控制）
```

鸿蒙能力映射（以 HarmonyOS NEXT / API 12+ 为准，具体包名以当前 SDK 文档为准）：

| 需求 | 鸿蒙 Kit / 模块 |
|---|---|
| HTTP 请求 | `@kit.NetworkKit` 的 `http.createHttp()`（**无自动 Cookie Jar，需自管 Cookie**，见 §3.4） |
| AES/MD5/RSA | `@kit.CryptoArchitectureKit` 的 `cryptoFramework`（AES-128-CBC/ECB + PKCS7、MD5 均有；**RSA NoPadding 是关键风险点**，见 §2.2.3） |
| 音频播放 | `@kit.MediaKit` 的 `media.AVPlayer`（支持 http(s) 流式 URL） |
| 后台/锁屏控制 | `@kit.AVSessionKit`（AVSession） |
| 设置存储 | `@kit.ArkData` 的 `preferences` |
| 文件读写 | `@kit.CoreFileKit` 的 `fileIo`（`context.filesDir` / `context.cacheDir`） |
| 二维码生成 | ohpm 三方库（zxing 移植等，以实际可用为准；内容就是一个 URL 字符串，ECC 等级 Low 即可） |
| 网络权限 | `module.json5` → `requestPermissions` 加 `ohos.permission.INTERNET` |

---

## 2. 网络与加密层（核心，务必最先实现并自测）

### 2.1 请求契约

- **除图片/歌曲文件下载用裸 GET 外，所有接口均为 POST**，`Content-Type: application/x-www-form-urlencoded`。
- Base URL：`https://music.163.com`；**eapi 类接口实际发往 `https://interface.music.163.com`**（见 §2.3）。
- 加密方式只有两种在用：**weapi**（绝大多数接口）与 **eapi**（仅播放地址相关接口）。crate 里还有 linuxapi 实现，但没有任何业务接口使用，可不实现。
- 成功判定：响应 JSON 的 `code == 200`（个别接口另有约定，见各接口条目）。
- 建议超时 10~30 秒（crate 用 100 秒，移动端不必照搬）。

### 2.2 weapi 加密（二次 AES-CBC + 裸 RSA）

适用：除 §2.3 列出的 eapi 接口外的所有 POST。

常量（原样复制）：

```
IV          = "0102030405060708"        // AES-CBC 初始向量，16 字节 ASCII
PRESET_KEY  = "0CoJUm6Qyw8W8jud"        // 第一重 AES-128 密钥
BASE62      = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
RSA 公钥（PKCS#8 PEM，1024 位，exponent 65537）：
-----BEGIN PUBLIC KEY-----
MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDgtQn2JZ34ZC28NWYpAUd98iZ37BUrX/aKzmFbt7clFSs6sXqHauqKWqdtLkF2KexO40H1YTX8z2lSgBBOAxLsvaklV8k4cBFK9snQXE9/DDaFt6Rr7iVZMldczhC0JNgTz+SHXT6CBHuX3e9SdB1Ua44oncaTWz7OBGLbCiK45wIDAQAB
-----END PUBLIC KEY-----
```

算法（输入 `text` 为参数字典的 JSON 序列化字符串）：

1. 生成 16 个随机字节，每字节 `b` 取 `BASE62[b % 62]`，得到 16 字符随机密钥 `secretKey`。
2. `params1 = base64(AES128_CBC_PKCS7(text, PRESET_KEY, IV))`
3. `params  = base64(AES128_CBC_PKCS7(params1, secretKey, IV))`（对 base64 字符串本身再加密一次）
4. `encSecKey = hex_lower(RSA_NOPADDING_ENCRYPT(reverse(secretKey), RSA公钥))`
   - `reverse` 是字符串反转（16 字符倒序）。
   - RSA 输入**左侧补 0x00 至 128 字节**（模数长度），**无任何填充方案（NoPadding）**，输出 128 字节转小写 hex（256 字符）。
5. POST body：`params=<urlencode(params)>&encSecKey=<encSecKey>`。

注意：JSON 序列化的**字段顺序无所谓**（服务端不校验顺序）；每次请求都重新生成 `secretKey`，相同输入输出必然不同。

### 2.3 eapi 加密（AES-128-ECB + MD5 签名）

适用接口（当前仅播放地址）：
- `/api/song/enhance/player/url`（旧版，按码率）
- `/api/song/enhance/player/url/v1`（桌面版在用，按音质等级）

常量：

```
EAPIKEY = "e82ckenh8dichen8"   // AES-128-ECB 密钥
分隔符  = "-36cd479b6b5-"      // 字面量
```

算法（`path` 为原始 `/api/...` 路径，`text` 为参数 JSON）：

1. `message = "nobody" + path + "use" + text + "md5forencrypt"`
2. `digest  = md5_hex_lower(message)`
3. `data    = path + "-36cd479b6b5-" + text + "-36cd479b6b5-" + digest`
4. `params  = hex_UPPER(AES128_ECB_PKCS7(data, EAPIKEY))`（**大写 hex**）
5. 请求 URL：`https://interface.music.163.com` + `path.replaceFirst("/api", "/eapi")`，**不带 query 串**。
6. POST body：`params=<大写hex>`。

要点：**digest 里用的是 `/api/...` 原路径，但请求发往 `/eapi/...`**——这是最常见的实现错误。

eapi 的参数 JSON 里还要额外注入一个 `header` 对象（weapi 不需要）：

```json
{
  "header": {
    "osver": "16.2",
    "deviceId": "",
    "os": "iPhone OS",
    "appver": "9.0.90",
    "versioncode": "140",
    "mobilename": "",
    "buildver": "<当前毫秒时间戳字符串，取前至多 10 位>",
    "resolution": "1920x1080",
    "__csrf": "<当前 csrf，无则空串>",
    "channel": "",
    "requestId": "<毫秒时间戳>_<0000~0999 的四位随机数>"
  }
}
```

### 2.4 请求头与 Cookie 模板

每个 POST 带以下硬编码头：

```
Accept: */*
Accept-Language: en-US,en;q=0.5
Connection: keep-alive
Content-Type: application/x-www-form-urlencoded
Host: music.163.com                # eapi 时为 interface.music.163.com
Referer: https://music.163.com
User-Agent: <见 UA 池规则>
Cookie: <手工构造段>;<登录 Cookie 段>
```

**Cookie 头 = 手工随机段 + 登录态段**，两部分缺一不可：

手工随机段（**每次请求重新随机生成**，模板原样）：

```
os={os}; appver={appver}; osver={osver}; deviceId=; WEVNSM=1.0.0; WNMCID={wnmcid}; _ntes_nnid={nnid}; _ntes_nuid={nuid}; NMTID={nmtid}; __remember_me=true; channel=
```

| 字段 | eapi | 其他（weapi） |
|---|---|---|
| `os` | `iphone` | `pc` |
| `appver` | `9.0.90` | `2.7.1.198277` |
| `osver` | `16.2` | `10` |

随机值生成规则（`hex(u64)` 指随机 u64 的十六进制小写）：
- `nuid = hex(u64) + hex(u64)`
- `nnid = "{nuid},{当前毫秒}"`（即 `_ntes_nnid`）
- `nmtid = hex(u64)`
- `wnmcid = "{3 字节 hex}.{当前毫秒}"`

登录态段：登录成功后服务端下发的 `MUSIC_U`、`__csrf` 等 Cookie，由你自己的 Cookie Store 按 `music.163.com` 域追加（见 §4.4）。未登录时为空。

**csrf 处理**：登录后（或任何响应 set-cookie 后）从 Cookie Store 读 `music.163.com` 域下的 `__csrf` 值并缓存。之后：
- 所有 weapi/eapi 请求的参数 JSON 里注入 `"csrf_token": "<csrf>"`（无则空串）；
- 大多数 weapi 接口还要在 URL 上拼 `?csrf_token=<csrf>`（`append_csrf=true`）；例外接口：`/weapi/v1/artist/{id}`、`/weapi/v1/artist/songs`、`/api/album/sub|unsub`（`append_csrf=false`）。eapi 不拼 query。

**UA 池规则**：备 14 个 UA（见附录 A）。`mobile` 策略 = 索引 `rand % 7`；`pc` 策略 = 索引 `rand % 5 + 8`；默认 = 索引 `rand % 14`。crate 现状是所有接口都用默认（随机），仅 logout 固定 `pc`。鸿蒙版直接默认随机即可。

### 2.5 鸿蒙实现要点与自测

1. **先实现 AES/MD5/RSA 原语自测**（用标准测试向量），再实现 weapi/eapi。
2. **RSA NoPadding 是唯一风险点**：`cryptoFramework` 的 RSA 支持以 `RSA1024|NoPadding` 创建 Cipher，公钥用 `createAsyKeyGenerator('RSA1024').convertKey(derBytes, null)` 导入（PEM 去头尾后 base64 decode 即 DER）。加密输入必须恰好 128 字节（左侧补零）。若 SDK 版本对 NoPadding 加密有限制，回退方案是自实现 1024 位模幂（`m^e mod n`，e=65537），输入输出同为 128 字节大端——算法简单但需一个大数实现。
3. **联调冒烟用例**（未登录即可验证）：
   - weapi：`banners()`（POST `/weapi/v2/banner/get`，参数 `clientType=pc`）→ 返回 `code==200` 且 `banners[]` 非空。
   - eapi：`songs_url_v1([1908049566], "standard")` → 返回 `code==200` 且 `data[0].url` 非空。

---

## 3. 数据模型

ArkTS 定义（字段名可自定义，注释里的 JSON 路径必须按此解析）。`u64` 用 `number` 即可（远小于 2^53）。**所有缺失字符串一律回落 `"unknown"`，`picUrl` 类回落 `""`**；数组缺失回落 `[]`。

### 3.1 歌曲 `SongInfo`（核心模型）

```ts
export interface SongInfo {
  id: number;          // 歌曲 id
  name: string;
  singer: string;      // 只取第一个歌手名
  album: string;
  albumId: number;
  picUrl: string;
  duration: number;    // 毫秒
  songUrl: string;     // 客户端后填，初始 ''
  copyright: SongCopyright;  // 版权/可播状态
  quality: SongQualityState; // 音质状态（客户端维护）
}
```

不同接口返回的 JSON 结构不同，按下表按来源解析：

| 来源 | 数组路径 | id/name | singer | album / albumId / picUrl | duration |
|---|---|---|---|---|---|
| 歌单详情/单曲详情 | `songs[]`（空则 `playlist.tracks[]`） | `id`/`name` | `ar[0].name` | `al.name` / `al.id` / `al.picUrl` | `dt` |
| 云盘 | `data[]` | `songId`/`songName` | `artist` | `album` / `0` / `""` | `simpleSong.dt` |
| 私人 FM | `data[]` | `id`/`name` | `artists[0].name` | `album.name/id/picUrl` | `duration` |
| 每日推荐 | `data.dailySongs[]` | `id`/`name` | `ar[0].name` | `al.*` | `dt` |
| 搜索（单曲/歌词） | `result.songs[]` | `id`/`name` | `artists[0].name` | `album.*` | `duration` |
| 专辑详情 | `songs[]` | `id`/`name` | `ar[0].name` | **顶层** `album.name/id/picUrl` | `dt` |
| 歌手热门 50 | `hotSongs[]` | `id`/`name` | 顶层 `artist.name` | `al.name` / `al.id` / `""` | `dt` |
| 歌手全部歌曲 | `songs[]` | `id`/`name` | `ar[0].name` | `al.*` | `dt` |
| 电台节目 | `programs[]` | `mainTrackId`/`name` | 固定 `"第 N 期"`（N 为倒数序号） | album=`createTime` 转字符串 / `0` / `coverUrl` | `duration` |
| 心动模式 | `data[]` | `songInfo.id/name` | `songInfo.ar[0].name` | `songInfo.al.*` | `songInfo.dt` |

### 3.2 版权与音质

```ts
// fee: 0 免费 / 1 VIP / 4 付费专辑 / 8 VIP高码率；st<0 直接不可播
export enum SongCopyright { Free, VipOnly, Payment, VipOnlyHighRate, Unavailable, Unknown }
// 解析规则：privilege.st < 0 → Unavailable；否则按 privilege.fee（或歌曲 fee）映射；其他 → Unknown
// 可播判定 playable() = copyright !== Unavailable（桌面版默认过滤灰歌，可用设置放开）
```

```ts
export enum SongQuality { Standard, Higher, Extreme, Lossless, HiRes, Surround, AudioVivid, Master }
```

音质 ↔ 接口参数 ↔ 码率映射（两个方向都要实现）：

| 枚举 | `level` 字符串（请求参数） | `encodeType` | 按码率 `br` 反推区间（bps） | 桌面版名义码率（缓存文件名/回退显示用） |
|---|---|---|---|---|
| Standard | `standard` | `aac` | 0..=128000 | 128000 |
| Higher | `higher` | `aac` | 128001..=192000 | 192000 |
| Extreme | `exhigh` | `aac` | 192001..=320000 | 320000 |
| Lossless | `lossless` | `flac` | 320001..=999000 | 999000 |
| HiRes | `hires` | `flac` | 999001..=1900000 | 1900000 |
| Surround | `jyeffect` | `flac` | 1900001..=2695683 | 804505 |
| AudioVivid | `sky` | `flac`（另加参数 `immerseType=c51`） | 2695684..=4532510 | 2695684 |
| Master | `jymaster` | `flac` | 其余 | 4532511 |

```ts
export interface SongUrl {
  id: number;
  url: string;     // 可为空 → 服务端无权限下发；解析时直接丢弃空 url 条目
  rate: number;    // ← br，精确码率 bps
  quality: SongQuality; // 优先 ← level 字符串反推；失败再按 br 区间反推
}

export interface SongQualityState {
  available: SongQuality[];   // 默认 [Standard]
  selected: SongQuality | null; // 用户单曲指定，默认 null（用全局设置）
  actual: SongQuality | null;   // 实际拿到的音质
}
```

### 3.3 其余模型

```ts
export interface SongList {   // 歌单/专辑/电台的统一卡片模型
  id: number; name: string; coverImgUrl: string; author: string;
}
// 封面/作者字段按来源：用户歌单/榜单歌单 playlists[] → coverImgUrl + creator.nickname；
// 推荐 recommend[] → picUrl + creator.nickname；专辑 albums[]/data[] → picUrl + artist.name（收藏专辑取 artists[0].name）；
// 搜索歌单 result.playlists[]；搜索专辑 result.albums[] → picUrl + artist.name；电台 djRadios[] → picUrl + dj.nickname

export interface SingerInfo {
  id: number; name: string; picUrl: string; // ← img1v1Url；若以 "5639395138885805.jpg" 结尾（网易默认头像）置 ""
} // 数组源：result.artists[]

export interface PlayListDetail {
  id: number; name: string; coverImgUrl: string; description: string;
  createTime: number; trackUpdateTime: number;
  songs: SongInfo[];
} // ← playlist.*；songs ← songs[]（空则 playlist.tracks[]），并与同响应 privileges[] 按下标一一对应得出每首 copyright

export interface PlayListDetailDynamic { // 可能缺省
  subscribed: boolean; bookedCount: number; playCount: number; commentCount: number;
} // ← 顶层同名字段（bookedCount 即收藏数）

export interface AlbumDetail {
  id: number; name: string; picUrl: string; description: string; publishTime: number;
  artistId: number; artistName: string; artistPicUrl: string; // ← album.artist.*
  songs: SongInfo[]; // ← songs[]，每首 copyright ← 歌曲内嵌 privilege
}

export interface AlbumDetailDynamic { isSub: boolean; subCount: number; commentCount: number; } // ← isSub/subCount/commentCount

export interface LoginInfo {
  code: number;
  uid: number;       // ← profile.userId
  nickname: string;  // ← profile.nickname
  avatarUrl: string; // ← profile.avatarUrl
  vipType: number;   // ← profile.vipType（0 普通，11 黑胶）；桌面版 vipType≠0 时昵称前加 👑
  msg: string;       // 失败时的错误消息
}

export interface BannerInfo {
  pic: string;       // ← imageUrl
  targetId: number;  // ← targetId
  targetType: 'song' | 'album' | 'unknown'; // ← targetType：1→song，10→album，其他 unknown
} // 数组源 banners[]

export interface TopList {
  id: number; name: string; updateFrequency: string; description: string; coverImgUrl: string;
} // 数组源 list[]

export interface Lyrics {
  lyric: string[];   // ← lrc.lyric 按 \n 切分，去空行
  tlyric: string[];  // ← tlyric.lyric 同处理（可为空）
}

export interface Msg { code: number; msg: string; } // ← code + msg 或 message
```

---

## 4. 接口清单

约定：除标注 GET 外均为 POST 表单；除"加密"列标注 eapi 外均为 weapi；URL 均在 `https://music.163.com` 下（eapi 见 §2.3）；除标注外 URL 均拼 `?csrf_token=`。参数全为字符串（数值先转字符串）。`[可选]` = 桌面版未用或低频，二期再实现。

### 4.1 登录与用户

| # | 方法 | 路径 | 参数 | 返回 |
|---|---|---|---|---|
| 1 | `loginQrCreate()` | `/weapi/login/qrcode/unikey` | `type=1` | `unikey`；二维码内容 = `https://music.163.com/login?codekey={unikey}` |
| 2 | `loginQrCheck(key)` | `/weapi/login/qrcode/client/login` | `type=1`, `key=unikey` | `Msg`（code 状态机见 §5.1）；803 时响应 set-cookie 携带登录态 |
| 3 | `captcha(ctcode, phone)` | `/weapi/sms/captcha/sent` | `cellphone=phone`, `ctcode`, `secrete=music_middleuser_pclogin` | code 200 或 `data==true` 即成功 |
| 4 | `loginCellphone(ctcode, phone, captcha)` | `/weapi/w/login/cellphone` | `phone`, `countrycode=ctcode`, `type=1`, `https=true`, `remember=true`, `captcha` | `LoginInfo` |
| 5 | `loginStatus()` | `/api/nuser/account/get`（仍走 weapi 加密直发 music.163.com） | 无 | `LoginInfo`；未登录时 code 非 200 |
| 6 | `logout()` | `/weapi/logout`（UA 用 pc 策略） | 无 | 忽略返回，本地清 Cookie 即可 |
| 7 | `userSongIdList(uid)` | `/weapi/song/like/get` | `uid` | `ids[]`（"我喜欢"的歌曲 id 集合） |
| 8 | `userSongList(uid, offset, limit)` | `/weapi/user/playlist` | `uid`, `offset`, `limit` | `SongList[]` ← `playlist[]`；**第 0 个固定是"我喜欢的音乐"歌单** |
| 9 | `albumSublist(offset, limit)` | `/weapi/album/sublist` | `total=true`, `offset`, `limit` | `SongList[]` ← `data[]`（收藏的专辑） |
| 10 | `userCloudDisk()` | `/weapi/v1/cloud/get` | `offset=0`, `limit=10000` | `SongInfo[]`（云盘来源解析） |
| 11 | [可选] `login(username, password)` | 11 位纯数字 → `/weapi/login/cellphone`（参数 `phone`,`password`,`rememberLogin=true`）；否则 → `/weapi/login`（参数 `username`,`password`,`rememberLogin=true`,`clientToken=1_jVUMqWEPke0/1/Vu56xCmJpo5vP1grjn_SOVVDzOc78w8OKLVZ2JH7IfkjSXqgfmh`） | `LoginInfo`（密码需先 MD5——见备注） |
| 12 | [可选] `dailyTask()` | `/weapi/point/dailyTask` | `type=0` | `Msg` |

> #11 备注：桌面版未用密码登录；网易惯例 password 字段是明文密码的 MD5(hex)。如需实现请抓包验证。

### 4.2 播放地址与歌词

| # | 方法 | 路径 | 加密 | 参数 | 返回 |
|---|---|---|---|---|---|
| 13 | `songsUrlV1(ids, level)` | `/api/song/enhance/player/url/v1` | **eapi** | `ids`（JSON 数组串，如 `[1908049566]`）, `level`（见 §3.2 表）, `encodeType`（前三档 `aac` 其余 `flac`）, 仅 AudioVivid 加 `immerseType=c51` | `SongUrl[]` ← `data[]`；**空 url 条目被丢弃 → 空数组 = 无权限/下架** |
| 14 | `songLyric(id)` | `/weapi/song/lyric` | weapi | `id`, `lv=-1`, `tv=-1`, `csrf_token` | `Lyrics` |
| 15 | [可选] `songsUrl(ids, br)` | `/api/song/enhance/player/url` | **eapi** | `ids`, `br`（128000/192000/320000/999000/1900000） | 同 #13（旧版，桌面版已弃用） |

### 4.3 歌单 / 专辑 / 榜单 / 歌手

| # | 方法 | 路径 | 参数 | 返回 |
|---|---|---|---|---|
| 16 | `songListDetail(id)` | `/weapi/v6/playlist/detail` | `id`, `offset=0`, `total=true`, `limit=1000`, `n=1000`, `csrf_token` | `PlayListDetail`（含全量歌曲与逐首版权） |
| 17 | `songListDetailDynamic(id)` | `/weapi/playlist/detail/dynamic` | `id` | `PlayListDetailDynamic`（是否已收藏/收藏数/播放数/评论数） |
| 18 | `songsDetail(ids)` | `/weapi/v3/song/detail` | `c` = `[{\"id\":\"123\"},{\"id\":\"456\"}]`（**字面反斜杠转义的类 JSON 串**，见附录 B） | `SongInfo[]` ← `songs[]` |
| 19 | `album(albumId)` | `/weapi/v1/album/{id}`（id 内插路径） | 无 | `AlbumDetail` |
| 20 | `albumDetailDynamic(albumId)` | `/weapi/album/detail/dynamic` | `id` | `AlbumDetailDynamic` |
| 21 | `topSongList(cat, order, offset, limit)` | `/weapi/playlist/list` | `cat`（`全部`/`华语`/`欧美`/`日语`/`韩语`/`粤语`/`小语种`/`流行`/`摇滚`/`民谣`/`电子`/`舞曲`/`说唱`/`轻音乐`/`爵士`/`乡村`/`R&B/Soul`/`古典`/`民族`/`英伦`/`金属`/`朋克`/`蓝调`/`雷鬼`/`世界音乐`/`拉丁`/`另类/独立`/`New Age`/`古风`/`后摇`/`Bossa Nova`/`清晨`/`夜晚`/`学习`/`工作`/`午休`/`下午茶`/`地铁`/`驾车`/`运动`/`旅行`/`散步`/`酒吧`/`怀旧`/`清新`/`浪漫`/`性感`/`伤感`/`治愈`/`放松`/`孤独`/`感动`/`兴奋`/`快乐`/`安静`/`思念`/`影视原声`/`ACG`/`儿童`/`校园`/`游戏`/`70后`/`80后`/`90后`/`网络歌曲`/`KTV`/`经典`/`翻唱`/`吉他`/`钢琴`/`器乐`/`榜单`/`00后`）, `order`（`hot`/`new`）, `total=true`, `offset`, `limit` | `SongList[]` ← `playlists[]` |
| 22 | [可选] `topSongListHighquality(cat, lasttime, limit)` | `/api/playlist/highquality/list`（weapi 加密） | `cat`, `total=true`, `lasttime`（上一页末条 updateTime，首页 0）, `limit` | `SongList[]`（精品歌单） |
| 23 | `toplist()` | `/api/toplist`（weapi 加密） | 无 | `TopList[]` ← `list[]`（官方榜单汇总，榜单详情直接用 #16 传榜单 id） |
| 24 | `newAlbums(area, offset, limit)` | `/weapi/album/new` | `area`（`ALL`/`ZH`/`EA`/`KR`/`JP`）, `offset`, `limit`, `total=true` | `SongList[]` ← `albums[]` |
| 25 | `singerSongs(id)` | `/weapi/v1/artist/{id}`（**不拼 csrf query**） | 无 | `SongInfo[]` ← `hotSongs[]`（热门 50） |
| 26 | [可选] `singerAllSongs(id, order, offset, limit)` | `/weapi/v1/artist/songs`（不拼 csrf query） | `id`, `private_cloud=true`, `work_type=1`, `order`（`hot`/`time`）, `offset`, `limit` | `SongInfo[]` ← `songs[]` |
| 27 | `banners()` | `/weapi/v2/banner/get` | `clientType=pc` | `BannerInfo[]` ← `banners[]` |
| 28 | [可选] `recommendResource()` | `/weapi/v1/discovery/recommend/resource` | 无 | `SongList[]` ← `recommend[]`（每日推荐歌单，需登录） |
| 29 | `recommendSongs()` | `/api/v3/discovery/recommend/songs`（weapi 加密） | `afresh=false` | `SongInfo[]` ← `data.dailySongs[]`（每日推荐歌曲，需登录） |
| 30 | [可选] `homepage()` | `/api/homepage/block/page`（weapi 加密） | `refresh=false`, `cursor=null` | 原始 JSON（首页区块） |

### 4.4 搜索

统一入口：`search(keywords, type, offset, limit)` → POST `/weapi/search/get`，参数 `s=keywords`, `type`, `offset`, `limit`，返回原始 JSON 再按类型解析。

| 方法 | type | 解析 |
|---|---|---|
| `searchSong` | 1 | `SongInfo[]` ← `result.songs[]` |
| `searchAlbum` | 10 | `SongList[]` ← `result.albums[]` |
| `searchSinger` | 100 | `SingerInfo[]` ← `result.artists[]` |
| `searchSonglist` | 1000 | `SongList[]` ← `result.playlists[]` |
| `searchLyrics` | 1006 | `SongInfo[]` ← `result.songs[]`（按歌词搜单曲） |
| [可选] 用户/MV/电台/视频 | 1002 / 1004 / 1009 / 1014 | 桌面版未用 |

分页约定（桌面版）：首次 `offset=0, limit=50`；滚动到底且 `offset % 50 == 0` 时再取下一页，`offset += 本批实际条数`。

### 4.5 收藏（isLike=true 收藏 / false 取消）

| # | 方法 | 路径 | 参数 | 成功判定 |
|---|---|---|---|---|
| 31 | `likeSong(isLike, songId)` | `/weapi/radio/like` | `alg=itembased`, `trackId=songId`, `like=true/false`, `time=25` | code==200 |
| 32 | `likeSongList(isLike, id)` | 收藏 → `/weapi/playlist/subscribe`；取消 → `/weapi/playlist/unsubscribe` | `id` | code==200 |
| 33 | `likeAlbum(isLike, id)` | 收藏 → `/api/album/sub?id={id}`；取消 → `/api/album/unsub?id={id}`（**id 直接拼 query，不拼 csrf**） | `id`（body 里也给一份） | code==200 |

### 4.6 电台 / FM / 心动模式

| # | 方法 | 路径 | 参数 | 返回 |
|---|---|---|---|---|
| 34 | `userRadioSublist(offset, limit)` | `/weapi/djradio/get/subed` | `total=true`, `offset`, `limit` | `SongList[]` ← `djRadios[]`（订阅的电台） |
| 35 | `radioProgram(rid, offset, limit)` | `/weapi/dj/program/byradio` | `radioId`, `offset`, `limit`, `asc=false` | `SongInfo[]` ← `programs[]`（电台节目来源解析） |
| 36 | `playmodeIntelligenceList(sid, pid)` | `/weapi/playmode/intelligence/list` | `songId=sid`, `type=fromPlayOne`, `playlistId=pid`, `startMusicId=sid`, `count=1` | `SongInfo[]` ← `data[]`（心动模式推荐，插入当前曲后播放） |
| 37 | [可选] `personalFm()` | `/weapi/v1/radio/get` | 无 | `SongInfo[]` ← `data[]`（私人 FM） |
| 38 | [可选] `fmTrash(songId)` | `/weapi/radio/trash/add` | `alg=RT`, `songId`, `time=25` | code==200（FM 垃圾桶） |

### 4.7 文件下载（裸 GET，不走加密层）

| # | 方法 | 说明 |
|---|---|---|
| 39 | 图片下载 | GET `{picUrl}?param={宽}y{高}`（网易图片 CDN 缩放参数）；**本地文件已存在则跳过**（文件即缓存）。建议补齐 Referer/UA 头 |
| 40 | 歌曲下载 | GET `SongUrl.url` 落盘（桌面版实际用播放管线 download 模式边播边存，见 §6.3；鸿蒙用 AVPlayer 时可在播完后补一次 GET 缓存，或直接边下边播自实现） |

---

## 5. 登录流程实现

### 5.1 二维码登录（主方式）

```
[打开登录弹窗] → loginQrCreate()
     → 用 "https://music.163.com/login?codekey={unikey}" 生成二维码图（ECC Low，边长 ~140px 即可）
     → 启动轮询：每 1 秒调一次 loginQrCheck(unikey)
        code=800  二维码过期      → 二维码置灰 + 显示"刷新"按钮，停止轮询
        code=801  等待扫码        → 继续
        code=802  已扫码待确认    → 提示一次"已扫码，等待确认"，继续
        code=803  登录成功        → 见下方"登录成功收尾"
        其他 code                → 停止轮询（保底）
```

工程细节（桌面版行为，建议照搬）：

- **防重复轮询**：记录当前 `unikey`，仅当新 unikey 与轮询中的不同才启动新循环；循环每次迭代先校验自己的 unikey 是否仍是"当前 key"（被刷新顶替的旧循环自行退出）。
- 刷新按钮、弹窗重新打开时都重新走 `loginQrCreate`。
- 轮询用鸿蒙的 `setInterval`/任务调度即可，注意弹窗销毁时停轮询。

### 5.2 手机号 + 验证码登录（备选 Tab）

1. 输入区号 `ctcode`（默认 `86`）与手机号 → `captcha(ctcode, phone)` 发短信验证码，成败各弹提示。
2. 输入验证码 → `loginCellphone(ctcode, phone, captcha)` → 成功后同走"登录成功收尾"，失败提示"登录失败"。

### 5.3 登录成功收尾（统一入口 `checkLogin`）

1. 从 Cookie Store 取当前登录态 Cookie（`MUSIC_U`、`__csrf` 等）。
2. 用带登录态的 client 调 `loginStatus()` 拿 `LoginInfo`（uid/昵称/头像/vipType）。
3. **持久化 Cookie**：把 `music.163.com` 域下全部 Cookie 以 JSON 存入应用文件目录（如 `filesDir/cookies.json`），写入时统一补 `Domain=music.163.com; Path=/; Max-Age=31536000`。
4. 更新 UI：昵称（vipType≠0 加 👑）、头像（下载到缓存，50×50）、切换到"我的"页登录态。
5. 调 `userSongIdList(uid)` 拉"我喜欢"歌曲 id 集合缓存到内存（用于播放栏星标）。
6. 建议加一个"登录会话代次"计数器：每次登录/登出自增，所有登录相关异步回调先校验代次，过期会话的结果直接丢弃（防串号）。

### 5.4 启动恢复登录

1. App 启动正常加载发现页（`banners()` 成功）之后：若本地 `cookies.json` 存在，重建 Cookie Store 并发起 `checkLogin`（即再调 `loginStatus` 校验）。
2. `loginStatus` 失败 → 提示"登录失效"、清空登录态、**删除 cookies.json**。
3. 顺序注意：先发过一次正常请求（banners）再校验登录，可规避网易对冷启动校验请求的风控（桌面版如此，注释引用 Binaryify/NeteaseCloudMusicApi#1217）。

### 5.5 登出

调 `logout()`（可忽略其返回）→ 清 Cookie Store → 删 `cookies.json` → 清 uid/"我喜欢"集合/我的页内容 → 登录弹窗切回二维码 Tab。

### 5.6 Cookie Store 自管要点

鸿蒙 `http` 模块不会自动持久化/携带 Cookie，需要自己实现一个极简 Cookie Store：

- 每次响应解析 `Set-Cookie`（可能多条），按 `name` 覆盖存储，记录 `domain`（统一归到 `music.163.com`）与 `path`。
- 每次请求把 Store 里匹配域的 Cookie 拼到 §2.4 手工随机段之后。
- eapi 的 `interface.music.163.com` 与 `music.163.com` 视为同一域处理（桌面版 jar 即按 `music.163.com` 归一）。
- 关键登录 Cookie：`MUSIC_U`（登录凭证）、`__csrf`（csrf token 来源）。

---

## 6. 播放流程实现

### 6.1 音质设置

全局设置项 `music-rate`（uint，默认 0），索引 ↔ `SongQuality`：

```
0 Standard / 1 Higher / 2 Extreme / 3 Lossless / 4 HiRes / 5 Surround / 6 AudioVivid / 7 Master
```

单曲可被用户单独指定音质（`SongInfo.quality.selected`），优先于全局设置。

### 6.2 点击播放主链路

```
点击歌曲 AddPlay(si)
  → 加入播放队列（去重/置当前）
  → Play(si)：
      rate = si.quality.selected ?? 全局 music-rate
      cachePath = {cacheDir}/music_{si.id}_{名义码率(rate)}      // 名义码率见 §3.2 表，如 320000，无扩展名
      if cachePath 存在:                                          // 缓存命中，跳过 API
          si.songUrl = file://{cachePath}
          播放(si, bitrate = 名义码率)
      else:
          res = songsUrlV1([si.id], level(rate))
          if res 为空或失败:
              提示"获取播放链接失败" → 延迟 2 秒自动 PlayNextSong()
          else:
              si.songUrl = res[0].url; si.quality.actual = res[0].quality
              播放(si, bitrate = res[0].rate)                      // API 精确码率
播放(si, bitrate)：
  AVPlayer.stop() → 设置 url（http(s) 或 file://）→ 设音量 → play()
  更新播放栏 UI：封面/歌名/歌手/时长/码率标签（"{(bitrate+500)/1000} kbps"，0 则隐藏）
```

- **VIP 无权限**：服务端会下发空 url，解析层丢弃后得到空数组——按上表走"失败跳下一首"。
- **播放中错误 / 自然播完（EOS）**：均自动 `PlayNextSong()`。
- **灰歌过滤**：加入播放列表时按 `copyright.playable()` 过滤（`st<0` 的不可播），提供设置项放开。

### 6.3 歌曲缓存策略（桌面版行为，鸿蒙建议等效实现）

- 缓存文件命名：`music_{songId}_{名义码率}`（如 `music_1908049566_320000`），无扩展名，放应用缓存目录。
- **只缓存时长 > 30 秒的歌**（规避 VIP 30 秒试听片段被当成完整缓存）。
- 桌面版靠 GStreamer playbin 的 download 模式边播边存（播完回调把临时文件 copy 到缓存路径）。鸿蒙 AVPlayer 无等价回调时，二选一：
  1. 播放完成后后台静默 GET `songUrl` 落盘到缓存路径（简单、多耗一倍流量）；或
  2. 自建本地代理/流式转发，边下边播（省流量，复杂）。
- 建议加缓存上限与"清理缓存"设置项（桌面版有缓存清理设置）。

### 6.4 预取下一首

播放进度到**剩余 5 秒**时，对队列下一首提前调 `songsUrlV1` 并把 url 写回队列项（`si.songUrl` 非空则跳过请求），保证无缝切歌。

### 6.5 队列与循环模式

桌面版支持列表循环/单曲循环/随机/心动（心动 = 播完当前后调 #36 `playmodeIntelligenceList(当前曲id, 其 albumId)`，把返回曲目插入当前曲后继续）。状态持久化：退出时保存队列 + 当前曲 + 进度，下次启动恢复（恢复后首次播放重新走 `Play(si)` 刷新 url，因为 url 会过期）。

### 6.6 后台播放

接入 AVSession：上报媒体元数据（歌名/歌手/封面/时长）、进度，注册 播放/暂停/上一首/下一首/进度跳转 控制命令，保证切后台与锁屏可控。

---

## 7. 歌词流程

### 7.1 获取与解析

1. 切歌时调 `songLyric(id)`（#14），得到 `lyric[]`（原文）与 `tlyric[]`（翻译，可空）。
2. 本地缓存（可省略，但省流量）：目录 `{filesDir}/lyrics/`；主歌词文件名 `{歌名}-{歌手}-{专辑}.lrc`（歌名中 `/` 替换为全角 `／`），翻译 `{songId}.tlrc`。命中缓存则不请求 API。
3. 解析为 `(时间戳ms, 文本)` 列表：
   - 行首时间标签正则 `\[(\d+):(\d+)\.(\d+)\]`，取**前两位分、两位秒、两位百分秒**：`ms = (mm*60 + ss) * 1000 + cc * 10`。
   - 修复异常时间戳：`[mm:ss:ms]` → `[mm:ss.ms]`（正则 `^\[(\d+):(\d+):(\d+)\]` 替换为 `[$1:$2.$3]`），先修复再解析。
   - 不匹配时间标签的行丢弃。
4. **合并翻译**：对每行原文，若某翻译行与原文行**前 10 个字符相同**（即同一时间标签），则翻译行用原文行的时间戳，紧跟原文行插入结果列表。
5. 播放进度每 ~500ms 驱动一次高亮。

### 7.2 当前行高亮算法

- 在解析结果末尾追加两个哨兵行 `(3600000000, "")`。
- 滑窗扫 3 行 `(l0, l1, l2)`：`time ∈ [l0.t, l1.t)` → 当前行是 l0；**若 `l1.t == l0.t`**（同时间戳的翻译行）→ 当前行是 (l0, l1) 两行一起高亮。
- 已唱过的行用弱化样式，当前行高亮并滚动居中；用户手动滚动后 3 秒内暂停自动滚动。

---

## 8. 图片缓存规则

下载统一走 #39（GET `{url}?param={W}y{H}`，文件存在即跳过）。桌面版缓存文件命名约定（鸿蒙可照搬以保持缓存可迁移）：

| 用途 | 文件名 | 请求尺寸 |
|---|---|---|
| 发现页 Banner | `{targetId}-banner.jpg` | 1200×465 |
| 歌单/专辑卡片、详情头图、播放栏封面、托盘图标 | `{id}-songlist-200.jpg`（播放栏用 `{albumId}-songlist-200.jpg`） | 200×200 |
| 歌曲行封面 | `{songId}-song-48.jpg` | 48×48 |
| 歌手头像 | `{singerId}-singer.jpg` | 140×140 |
| 用户头像 | `avatar.jpg`（固定名） | 50×50 |

---

## 9. 页面数据加载一览（桌面版行为，供排期参考）

| 页面 | 调用 | 参数 |
|---|---|---|
| 发现页轮播 | `banners()` | —；点击按 targetType：song → `songsDetail([targetId])` 直接播放；album → `album(targetId)` + `albumDetailDynamic` 开详情 |
| 发现页推荐歌单 | `topSongList("全部", "hot", 0, 8)` | 失败 500ms 后自动重试 |
| 发现页新碟 | `newAlbums("ALL", 0, 8)` | 同上 |
| 榜单页 | `toplist()`；选中后 `songListDetail(id)` | — |
| 歌单详情页 | `songListDetail(id)` + `songListDetailDynamic(id)` | 详情含全量曲目 |
| 专辑详情页 | `album(id)` + `albumDetailDynamic(id)` | — |
| 我的页 | `userSongList(uid, 0, 11)`（`skip(1)` 去掉"我喜欢"取 10 条预览）；`albumSublist(0, 10)` | — |
| 每日推荐 | `recommendSongs()` | 需登录 |
| 我喜欢的音乐 | `userSongList(uid, 0, 1)` 取首个歌单 → `songListDetail(id)` | — |
| 云盘 | `userCloudDisk()` | — |
| 我的电台 | `userRadioSublist(0, 1001)`；详情 `radioProgram(id, 0, 1001)` | — |
| 搜索页 | §4.4 五种 | 首次 50 条，滚动分页 |

---

## 10. 实施顺序建议

1. **加密层 + 网络层**：AES/MD5/RSA 原语自测 → weapi/eapi → §2.5 两个冒烟用例通过。
2. **登录闭环**：二维码状态机 → checkLogin → Cookie 持久化 → 启动恢复。
3. **播放闭环**：`songsUrlV1` → AVPlayer 播 URL → 队列/切歌/失败跳转 → 缓存 → 预取。
4. **内容页**：发现页 → 榜单 → 歌单/专辑详情 → 搜索 → 我的页（需登录）。
5. **歌词**：解析/合并/高亮。
6. **增强**：收藏、心动模式、AVSession、缓存清理设置。

## 11. 验收自测清单

- [ ] 未登录 `banners()` 返回 200（验证 weapi）
- [ ] 未登录 `songsUrlV1([1908049566], Standard)` 返回非空 url（验证 eapi）
- [ ] 二维码 800/801/802/803 全状态流转正常；803 后 `loginStatus` 拿到 uid
- [ ] 杀进程重启后免登录恢复；服务端失效时正确清理本地 Cookie
- [ ] VIP 歌曲无 url 时提示并 2 秒后自动跳下一首
- [ ] 切音质后缓存文件名随之变化（`music_{id}_{rate}`）
- [ ] 歌词时间轴与播放进度对齐，翻译行与原文行同高亮
- [ ] 灰歌默认不出现在播放列表

---

## 附录 A：User-Agent 池（14 条，原样）

```
0  Mozilla/5.0 (iPhone; CPU iPhone OS 9_1 like Mac OS X) AppleWebKit/601.1.46 (KHTML, like Gecko) Version/9.0 Mobile/13B143 Safari/601.1
1  （同 0，重复一条）
2  Mozilla/5.0 (Linux; Android 5.0; SM-G900P Build/LRX21T) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/59.0.3071.115 Mobile Safari/537.36
3  Mozilla/5.0 (Linux; Android 6.0; Nexus 5 Build/MRA58N) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/59.0.3071.115 Mobile Safari/537.36
4  Mozilla/5.0 (Linux; Android 5.1.1; Nexus 6 Build/LYZ28E) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/59.0.3071.115 Mobile Safari/537.36
5  Mozilla/5.0 (iPhone; CPU iPhone OS 10_3_2 like Mac OS X) AppleWebKit/603.2.4 (KHTML, like Gecko) Mobile/14F89;GameHelper
6  Mozilla/5.0 (iPhone; CPU iPhone OS 10_0 like Mac OS X) AppleWebKit/602.1.38 (KHTML, like Gecko) Version/10.0 Mobile/14A300 Safari/602.1
7  Mozilla/5.0 (iPad; CPU OS 10_0 like Mac OS X) AppleWebKit/602.1.38 (KHTML, like Gecko) Version/10.0 Mobile/14A300 Safari/602.1
8  Mozilla/5.0 (Macintosh; Intel Mac OS X 10.12; rv:46.0) Gecko/20100101 Firefox/46.0
9  Mozilla/5.0 (Macintosh; Intel Mac OS X 10_12_5) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/59.0.3071.115 Safari/537.36
10 Mozilla/5.0 (Macintosh; Intel Mac OS X 10_12_5) AppleWebKit/603.2.4 (KHTML, like Gecko) Version/10.1.1 Safari/603.2.4
11 Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:46.0) Gecko/20100101 Firefox/46.0
12 Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/51.0.2704.103 Safari/537.36
13 Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/42.0.2311.135 Safari/537.36 Edge/13.1058
```

选择规则：`mobile` → `rand % 7`；`pc` → `rand % 5 + 8`；默认 → `rand % 14`。

## 附录 B：已知怪癖（原样保留，勿当笔误"修复"）

1. `captcha` 参数名是 `secrete`（不是 `secret`），值 `music_middleuser_pclogin`。
2. `songsDetail` 的 `c` 参数是**带字面反斜杠**的串：`[{\"id\":\"123\"}]`（即 JSON 字符串里再转义一层后的文本形态）。
3. eapi 的 MD5 digest 用 `/api/...` 原路径，但请求发往 `interface.music.163.com/eapi/...`。
4. eapi 输出**大写** hex，weapi 的 encSecKey 输出**小写** hex。
5. `SongUrl` 空 url 条目必须在解析层丢弃；因此空数组语义 = 无权限，不是网络错误。
6. 手工 Cookie 段的随机字段（`_ntes_nuid`/`_ntes_nnid`/`NMTID`/`WNMCID`）**每次请求重新生成**，不要缓存复用。
7. 登录态 Cookie 不在手工 Cookie 段里，由 Cookie Store 追加；两者顺序：手工段在前。
8. crate 的 `logout` 有 URL 双前缀拼接 bug（`https://music.163.comhttps://...`）；鸿蒙版直接 POST `https://music.163.com/weapi/logout` 即可，本地清 Cookie 才是关键动作。
9. 用户歌单列表第 0 个固定是"我喜欢的音乐"，"我的收藏歌单"展示时要 `skip(1)`。
10. `singerSongs` / `singerAllSongs` / `likeAlbum` 三个接口**不**拼 `?csrf_token=`；`likeAlbum` 的 id 直接出现在 URL query 里。
11. 播放地址 http(s) url 自带时效参数，恢复上次会话时必须重新取 url，不要持久化 url 本身。
12. 歌词只解析两位百分秒的时间标签（`[mm:ss.xx]`）；三位毫秒标签会被丢弃——先按 §7.1 的修复正则规范化。
