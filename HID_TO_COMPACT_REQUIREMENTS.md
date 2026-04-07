# HIDレポートからcompact独自インターフェースへの直接変換要件

## 目的

DUALSHOCK 4 の 10 バイトHIDレポートを、固定長 8 バイトの独自インターフェース `DS4_COMPACT_V1` へ直接変換する。

## 前提

- 入力は 10 バイトの HID レポート
- `hid[0]` は `report_id` で、値は `1` を想定する
- 出力は 8 バイトの `compact[0]..compact[7]`
- ビット順は `lsb0`
- `compact[0]` と `compact[1]` はデジタル入力
- `compact[2]` から `compact[7]` はアナログ入力

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

## HIDからcompactへの直接対応

### 1. D-pad

`hid[5] & 0x0F` を D-pad 値として読む。

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

- `compact[0]` bit0 = `hid[5] & 0x0F` が `{0, 1, 7}` のとき 1
- `compact[0]` bit1 = `hid[5] & 0x0F` が `{1, 2, 3}` のとき 1
- `compact[0]` bit2 = `hid[5] & 0x0F` が `{3, 4, 5}` のとき 1
- `compact[0]` bit3 = `hid[5] & 0x0F` が `{5, 6, 7}` のとき 1

### 2. フェイスボタン

5バイト目の上位4bitをそのまま `compact[0]` の bit4-bit7 へ写す。

- `compact[0]` bit4 = `(hid[5] & 0x10) != 0`
- `compact[0]` bit5 = `(hid[5] & 0x20) != 0`
- `compact[0]` bit6 = `(hid[5] & 0x40) != 0`
- `compact[0]` bit7 = `(hid[5] & 0x80) != 0`

### 3. ショルダー/システム/スティック押し込み

6バイト目と7バイト目から `compact[1]` を作る。

- `compact[1]` bit0 = `(hid[6] & 0x01) != 0`
- `compact[1]` bit1 = `(hid[6] & 0x02) != 0`
- `compact[1]` bit2 = `(hid[6] & 0x10) != 0`
- `compact[1]` bit3 = `(hid[6] & 0x20) != 0`
- `compact[1]` bit4 = `(hid[6] & 0x40) != 0`
- `compact[1]` bit5 = `(hid[6] & 0x80) != 0`
- `compact[1]` bit6 = `(hid[7] & 0x01) != 0`
- `compact[1]` bit7 = `(hid[7] & 0x02) != 0`

### 4. アナログ値

次のバイトは値をそのままコピーする。

- `compact[2] = hid[1]`
- `compact[3] = hid[2]`
- `compact[4] = hid[3]`
- `compact[5] = hid[4]`
- `compact[6] = hid[8]`
- `compact[7] = hid[9]`

## 擬似コード

```text
compact[0] = 0
compact[1] = 0
compact[2] = hid[1]
compact[3] = hid[2]
compact[4] = hid[3]
compact[5] = hid[4]
compact[6] = hid[8]
compact[7] = hid[9]

dpad = hid[5] & 0x0F

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

if hid[5] & 0x10: set compact[0].bit4
if hid[5] & 0x20: set compact[0].bit5
if hid[5] & 0x40: set compact[0].bit6
if hid[5] & 0x80: set compact[0].bit7

if hid[6] & 0x01: set compact[1].bit0
if hid[6] & 0x02: set compact[1].bit1
if hid[6] & 0x10: set compact[1].bit2
if hid[6] & 0x20: set compact[1].bit3
if hid[6] & 0x40: set compact[1].bit4
if hid[6] & 0x80: set compact[1].bit5
if hid[7] & 0x01: set compact[1].bit6
if hid[7] & 0x02: set compact[1].bit7
```

## エラー条件

- HID レポート長が 10 バイト未満
- D-pad 値が `0..8` 以外
- 出力先 `compact` が 8 バイト未満

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
