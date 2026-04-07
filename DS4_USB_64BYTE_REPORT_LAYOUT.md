# DUALSHOCK 4 USB 64バイト入力レポート構造メモ

## 目的

DUALSHOCK 4 の USB 接続時に得られる 64 バイト入力レポート全体の構造を整理する。

この文書は次の用途を想定する。

- 64 バイトレポートの各バイト範囲が何を表すかを把握する
- `HID_TO_COMPACT_REQUIREMENTS.md` で扱っている先頭 10 バイトが、全体のどこに位置するかを確認する
- 将来的にセンサー値、バッテリー状態、タッチパッド情報を使う際の参照にする

## 前提

- USB 入力レポート長は 64 バイト
- `report_id` は `0x01`
- Linux kernel の `hid-playstation.c` にある `struct dualshock4_input_report_usb` を基準に整理する
- マルチバイト値は little-endian

## 全体構造

USB 64 バイト入力レポートは、概ね次のように分かれる。

```text
byte  0      : report_id
byte  1..32  : common input report
byte 33      : num_touch_reports
byte 34..60  : touch_reports[3]
byte 61..63  : reserved
```

サイズ内訳は次の通り。

- `report_id`: 1 バイト
- `common`: 32 バイト
- `num_touch_reports`: 1 バイト
- `touch_reports[3]`: 27 バイト
- `reserved`: 3 バイト

合計 64 バイト。

## バイト単位レイアウト

| オフセット | 長さ | フィールド名 | 内容 |
| --- | ---: | --- | --- |
| `0` | 1 | `report_id` | 入力レポート ID。USB では `0x01` |
| `1` | 1 | `x` | 左スティック X (`lx`) |
| `2` | 1 | `y` | 左スティック Y (`ly`) |
| `3` | 1 | `rx` | 右スティック X |
| `4` | 1 | `ry` | 右スティック Y |
| `5` | 1 | `buttons[0]` | D-pad と face buttons |
| `6` | 1 | `buttons[1]` | L1/R1/Share/Options/L3/R3 |
| `7` | 1 | `buttons[2]` | PS、trackpad click ほか |
| `8` | 1 | `z` | `l2` アナログ値 |
| `9` | 1 | `rz` | `r2` アナログ値 |
| `10..11` | 2 | `sensor_timestamp` | センサー時刻 |
| `12` | 1 | `sensor_temperature` | センサー温度 |
| `13..14` | 2 | `gyro[0]` | ジャイロ X |
| `15..16` | 2 | `gyro[1]` | ジャイロ Y |
| `17..18` | 2 | `gyro[2]` | ジャイロ Z |
| `19..20` | 2 | `accel[0]` | 加速度 X |
| `21..22` | 2 | `accel[1]` | 加速度 Y |
| `23..24` | 2 | `accel[2]` | 加速度 Z |
| `25..29` | 5 | `reserved2` | 予約領域 |
| `30..31` | 2 | `status` | バッテリー状態、接続状態など |
| `32` | 1 | `reserved3` | 予約領域 |
| `33` | 1 | `num_touch_reports` | このレポートに含まれるタッチレポート数 |
| `34..42` | 9 | `touch_reports[0]` | タッチレポート 0 |
| `43..51` | 9 | `touch_reports[1]` | タッチレポート 1 |
| `52..60` | 9 | `touch_reports[2]` | タッチレポート 2 |
| `61..63` | 3 | `reserved` | 予約領域 |

## 先頭 10 バイトの意味

このプロジェクトで `compact` 変換に使っているのは、USB 64 バイト入力の先頭 10 バイトである。

```text
usb[0] = report_id
usb[1] = lx
usb[2] = ly
usb[3] = rx
usb[4] = ry
usb[5] = buttons[0]
usb[6] = buttons[1]
usb[7] = buttons[2]
usb[8] = l2
usb[9] = r2
```

したがって `HID_TO_COMPACT_REQUIREMENTS.md` で定義している `hid10` は、USB レポートでは単純に `usb[0..9]` に対応する。

## `buttons[0]..buttons[2]` の概要

### `buttons[0]`

- 下位 4bit: D-pad
- 上位 4bit: `square`, `cross`, `circle`, `triangle`

### `buttons[1]`

- bit 0: `l1`
- bit 1: `r1`
- bit 4: `share`
- bit 5: `options`
- bit 6: `l3`
- bit 7: `r3`

### `buttons[2]`

- bit 0: `ps`
- bit 1: `trackpad_click`
- それ以外のビットはこのプロジェクトでは未使用

## センサー領域

`sensor_timestamp` 以降は、主に IMU と状態情報で構成される。

- `sensor_timestamp`: センサー時刻
- `sensor_temperature`: 温度
- `gyro[3]`: ジャイロ 3 軸
- `accel[3]`: 加速度 3 軸
- `status[2]`: バッテリーや接続状態など

現時点の `compact` 変換では、この領域は使わない。

## タッチパッド領域

USB レポートには 3 件のタッチレポートが含まれる。各タッチレポートは 9 バイトで、構造は次の通り。

```text
u8 timestamp
touch_point[0] 4 bytes
touch_point[1] 4 bytes
```

各 touch point は 4 バイトで、X/Y 座標と接触状態を持つ。

```text
byte 0: contact
byte 1: x_lo
byte 2: x_hi(4bit), y_lo(4bit)
byte 3: y_hi
```

この文書では touch point のビット意味までは掘り下げず、USB 64 バイト全体の見取り図として扱う。

## 実装上の扱い

このプロジェクトでは、現時点で USB 64 バイト入力を次のように扱うのが妥当である。

1. `usb[0..9]` を `hid10` として切り出す
2. `hid10` を `compact` へ変換する
3. `usb[10..63]` は必要になるまで保持または無視する

この方針により、Bluetooth 10 バイト入力との共通化を最小コストで実現できる。

## 補足

- Bluetooth のフル入力レポートは USB と似た `common` 領域を持つが、ヘッダ、タッチレポート数、CRC の扱いが異なる
- 一部のサードパーティ製パッドでは Bluetooth 側で短い 10 バイトレポートのみが使われることがある
- 本文書は USB 64 バイト入力レポートに限定している

## 根拠

主な根拠は Linux kernel の DUALSHOCK 4 入力レポート定義である。

- `struct dualshock4_input_report_common`
- `struct dualshock4_input_report_usb`
- `struct dualshock4_touch_report`
- `struct dualshock4_touch_point`

参照:

- https://kernel.googlesource.com/pub/scm/linux/kernel/git/stable/linux-stable/+/master/drivers/hid/hid-playstation.c
