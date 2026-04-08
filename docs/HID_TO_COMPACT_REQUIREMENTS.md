# HIDレポートからcompact独自インターフェースへの直接変換要件

## 目的

DUALSHOCK 4 の HID レポートから、固定長 8 バイトの独自インターフェース `DS4_COMPACT_V1` へ直接変換する。

本要件書では次の 2 系統を対象にする。

- Bluetooth の 10 バイト入力レポート
- USB の 64 バイト入力レポート

USB の 64 バイト入力レポートについては、Bluetooth の 10 バイト入力レポートと同等の意味を持つ先頭 10 バイトを抽出し、その後の変換結果が Bluetooth の場合と一致することを目的とする。

## 前提

- Bluetooth 入力は 10 バイトの HID レポート
- USB 入力は 64 バイトの HID レポート
- Bluetooth/USB ともに、変換に使う論理入力は 10 バイトの共通表現 `hid10[0]..hid10[9]` として扱う
- `hid10[0]` は `report_id` で、値は `1` を想定する
- 出力は 8 バイトの `compact[0]..compact[7]`
- ビット順は `lsb0`
- `compact[0]` と `compact[1]` はデジタル入力
- `compact[2]` から `compact[7]` はアナログ入力

## 正規化方針

入力が Bluetooth の 10 バイトレポートか USB の 64 バイトレポートかにかかわらず、まず 10 バイトの共通表現 `hid10` に正規化してから `compact` へ変換する。

```text
Bluetooth 10 bytes -> hid10 -> compact
USB 64 bytes       -> hid10 -> compact
```

以降の変換仕様はすべて `hid10` を入力とした要件として定義する。

## `hid10` の定義

### Bluetooth 入力の場合

Bluetooth の 10 バイトレポートをそのまま `hid10` とする。

```text
hid10[0] = bt[0]
hid10[1] = bt[1]
hid10[2] = bt[2]
hid10[3] = bt[3]
hid10[4] = bt[4]
hid10[5] = bt[5]
hid10[6] = bt[6]
hid10[7] = bt[7]
hid10[8] = bt[8]
hid10[9] = bt[9]
```

### USB 入力の場合

USB の 64 バイトレポートから先頭 10 バイトを `hid10` として抽出する。

```text
hid10[0] = usb[0]
hid10[1] = usb[1]
hid10[2] = usb[2]
hid10[3] = usb[3]
hid10[4] = usb[4]
hid10[5] = usb[5]
hid10[6] = usb[6]
hid10[7] = usb[7]
hid10[8] = usb[8]
hid10[9] = usb[9]
```

USB レポートの `usb[10]` 以降は、本仕様で定義する `DS4_COMPACT_V1` 変換には使用しない。

## USB 64バイト入力と Bluetooth 10バイト入力の対応関係

カーネル実装で定義されている DUALSHOCK 4 の共通入力レイアウトでは、USB 64 バイトレポートは次の構造を持つ。

- `usb[0]`: `report_id`
- `usb[1]`: `x`
- `usb[2]`: `y`
- `usb[3]`: `rx`
- `usb[4]`: `ry`
- `usb[5]`: `buttons[0]`
- `usb[6]`: `buttons[1]`
- `usb[7]`: `buttons[2]`
- `usb[8]`: `z`
- `usb[9]`: `rz`

この先頭 10 バイトは、Bluetooth 側の 10 バイト入力と同じ論理項目を表している。したがって本要件では、次の 1 対 1 対応を採用する。

| 共通表現 `hid10` | Bluetooth 10 バイト | USB 64 バイト | 意味 |
| --- | --- | --- | --- |
| `hid10[0]` | `bt[0]` | `usb[0]` | `report_id` |
| `hid10[1]` | `bt[1]` | `usb[1]` | `lx` |
| `hid10[2]` | `bt[2]` | `usb[2]` | `ly` |
| `hid10[3]` | `bt[3]` | `usb[3]` | `rx` |
| `hid10[4]` | `bt[4]` | `usb[4]` | `ry` |
| `hid10[5]` | `bt[5]` | `usb[5]` | D-pad とフェイスボタン |
| `hid10[6]` | `bt[6]` | `usb[6]` | ショルダー/Share/Options/L3/R3 |
| `hid10[7]` | `bt[7]` | `usb[7]` | PS/trackpad_click ほか |
| `hid10[8]` | `bt[8]` | `usb[8]` | `l2` アナログ値 |
| `hid10[9]` | `bt[9]` | `usb[9]` | `r2` アナログ値 |

## USB から同等出力を得るための要件

- USB 入力を処理する実装は、必ず `usb[0..9]` を `hid10` にコピーしてから既存の Bluetooth 10 バイト変換ロジックへ渡すこと
- `usb[10]` 以降の追加情報に依存して `compact` の値を変えてはならない
- 同一の物理入力状態を Bluetooth 10 バイト入力と USB 64 バイト入力で表した場合、出力される `compact[0]..compact[7]` は一致しなければならない
- `report_id` が想定値でない場合の扱いは、Bluetooth と USB で同じ方針に統一すること
- USB 入力長が 64 バイト未満の場合は不正入力とする
- USB 入力長が 64 バイト以上でも、本仕様で参照するのは `usb[0]..usb[9]` のみとする

## 出力レイアウト

### `compact[0]`

- bit 0: `up`
- bit 1: `right`
- bit 2: `down`
- bit 3: `left`
- bit 4: `square`
- bit 5: `cross`
- bit 6: `circle`
- bit 7: `triangle`

### `compact[1]`

- bit 0: `l1`
- bit 1: `r1`
- bit 2: `share`
- bit 3: `options`
- bit 4: `l3`
- bit 5: `r3`
- bit 6: `ps`
- bit 7: `trackpad_click`

### `compact[2]..compact[7]`

- `compact[2]`: `lx`
- `compact[3]`: `ly`
- `compact[4]`: `rx`
- `compact[5]`: `ry`
- `compact[6]`: `l2`
- `compact[7]`: `r2`

## `hid10` から compact への直接対応

### 1. D-pad

`hid10[5] & 0x0F` を D-pad 値として読む。

対応は次の通り。

- `0`: `compact[0]` bit0 = 1
- `1`: `compact[0]` bit0 = 1, bit1 = 1
- `2`: `compact[0]` bit1 = 1
- `3`: `compact[0]` bit1 = 1, bit2 = 1
- `4`: `compact[0]` bit2 = 1
- `5`: `compact[0]` bit2 = 1, bit3 = 1
- `6`: `compact[0]` bit3 = 1
- `7`: `compact[0]` bit3 = 1, bit0 = 1
- `8`: `compact[0]` bit0-bit3 = 0

`0` から `8` 以外は不正値とする。

同じ内容を条件式で表すと次の通り。

- `compact[0]` bit0 = `hid10[5] & 0x0F` が `{0, 1, 7}` のとき 1
- `compact[0]` bit1 = `hid10[5] & 0x0F` が `{1, 2, 3}` のとき 1
- `compact[0]` bit2 = `hid10[5] & 0x0F` が `{3, 4, 5}` のとき 1
- `compact[0]` bit3 = `hid10[5] & 0x0F` が `{5, 6, 7}` のとき 1

### 2. フェイスボタン

`hid10[5]` の上位 4bit をそのまま `compact[0]` の bit4-bit7 へ写す。

- `compact[0]` bit4 = `(hid10[5] & 0x10) != 0`
- `compact[0]` bit5 = `(hid10[5] & 0x20) != 0`
- `compact[0]` bit6 = `(hid10[5] & 0x40) != 0`
- `compact[0]` bit7 = `(hid10[5] & 0x80) != 0`

### 3. ショルダー/システム/スティック押し込み

`hid10[6]` と `hid10[7]` から `compact[1]` を作る。

- `compact[1]` bit0 = `(hid10[6] & 0x01) != 0`
- `compact[1]` bit1 = `(hid10[6] & 0x02) != 0`
- `compact[1]` bit2 = `(hid10[6] & 0x10) != 0`
- `compact[1]` bit3 = `(hid10[6] & 0x20) != 0`
- `compact[1]` bit4 = `(hid10[6] & 0x40) != 0`
- `compact[1]` bit5 = `(hid10[6] & 0x80) != 0`
- `compact[1]` bit6 = `(hid10[7] & 0x01) != 0`
- `compact[1]` bit7 = `(hid10[7] & 0x02) != 0`

### 4. アナログ値

次のバイトは値をそのままコピーする。

- `compact[2] = hid10[1]`
- `compact[3] = hid10[2]`
- `compact[4] = hid10[3]`
- `compact[5] = hid10[4]`
- `compact[6] = hid10[8]`
- `compact[7] = hid10[9]`

## 擬似コード

```text
compact[0] = 0
compact[1] = 0
compact[2] = hid10[1]
compact[3] = hid10[2]
compact[4] = hid10[3]
compact[5] = hid10[4]
compact[6] = hid10[8]
compact[7] = hid10[9]

dpad = hid10[5] & 0x0F

if dpad == 0: set compact[0].bit0
if dpad == 1: set compact[0].bit0, bit1
if dpad == 2: set compact[0].bit1
if dpad == 3: set compact[0].bit1, bit2
if dpad == 4: set compact[0].bit2
if dpad == 5: set compact[0].bit2, bit3
if dpad == 6: set compact[0].bit3
if dpad == 7: set compact[0].bit3, bit0
if dpad == 8: set nothing
otherwise: error

if hid10[5] & 0x10: set compact[0].bit4
if hid10[5] & 0x20: set compact[0].bit5
if hid10[5] & 0x40: set compact[0].bit6
if hid10[5] & 0x80: set compact[0].bit7

if hid10[6] & 0x01: set compact[1].bit0
if hid10[6] & 0x02: set compact[1].bit1
if hid10[6] & 0x10: set compact[1].bit2
if hid10[6] & 0x20: set compact[1].bit3
if hid10[6] & 0x40: set compact[1].bit4
if hid10[6] & 0x80: set compact[1].bit5
if hid10[7] & 0x01: set compact[1].bit6
if hid10[7] & 0x02: set compact[1].bit7
```

## エラー条件

- Bluetooth レポート長が 10 バイト未満
- USB レポート長が 64 バイト未満
- D-pad 値が `0..8` 以外
- 出力先 `compact` が 8 バイト未満

## 根拠

USB 64 バイト入力と Bluetooth 入力の対応関係は、Linux kernel の `hid-playstation.c` にある DUALSHOCK 4 入力レポート構造定義を根拠とする。

- `struct dualshock4_input_report_common`
- `struct dualshock4_input_report_usb`
- `DS4_INPUT_REPORT_USB_SIZE == 64`
- `DS4_INPUT_REPORT_BT_MINIMAL_SIZE == 10`

参照:

- https://kernel.googlesource.com/pub/scm/linux/kernel/git/stable/linux-stable/+/master/drivers/hid/hid-playstation.c

## 変換例

### 入力

```text
hid = [0x11, 255, 128, 1, 127, 7, 0x52, 0x03, 255, 64]
```

### 解釈

- `hid[5] & 0x0F = 7` なので `up=1`, `left=1`
- `hid[6] = 0x52` なので `r1=1`, `share=1`, `l3=1`
- `hid[7] = 0x03` なので `ps=1`, `trackpad_click=1`
- アナログ値は `255, 128, 1, 127, 255, 64`

### 出力

```text
compact = [0x09, 0xD6, 0xFF, 0x80, 0x01, 0x7F, 0xFF, 0x40]
```
