# arm9 コントローラー入力と CAN 対応

`arm9` Manual モードで、DS4 の入力がどの CAN ID / Data byte に載るかを、デフォルト値と入力後の値つきで一覧化したメモです。

## 電流値として載る入力

| コントローラー入力 | CAN ID | Data | デフォルト値 | 入力後の値 |
| --- | --- | --- | --- | --- |
| `R2 / L2` | `0x200` | `Data[0..1]` | `255` | Normal: `155 / 355`、Power: `100 / 400`、Sensitive: `180 / 330` |
| `R1 / L1` | `0x200` | `Data[2..3]` | `255` | Normal: `315 / 205`、Power: `511 / 1`、Sensitive: `295 / 225` |
| 右スティック Y | `0x200` | `Data[4..5]` | `255` | 右スティック下/上。Normal: `230 / 280`、Power: `170 / 340`、Sensitive: `230 / 260` |
| 左スティック Y | `0x200` | `Data[6..7]` | `255` | 左スティック下/上。Normal: `225 / 275`、Power: `175 / 325`、Sensitive: `215 / 280` |
| `triangle / cross` | `0x201` | `Data[0..1]` | `255` | Normal: `210 / 400`、Power: `160 / 450`、Sensitive: `225 / 290` |
| `circle / square` | `0x201` | `Data[2..3]` | `255` | Normal: `190 / 310`、Power: `80 / 430`、Sensitive: `210 / 300` |
| `D-pad Right / Left` | `0x201` | `Data[4..5]` | `255` | Normal: `155 / 285`、Power: `105 / 335`、Sensitive: `240 / 270` |

## ビットとして載る入力

| コントローラー入力 | CAN ID | Data | デフォルト値 | 入力後の値 |
| --- | --- | --- | --- | --- |
| `D-pad Up` | `0x201` | `Data[6] bit3` | `0` | `1` |
| `D-pad Down` | `0x201` | `Data[6] bit4` | `0` | `1` |
| `r3` | `0x201` | `Data[6] bit5` | `0` | `1` |
| `l3` | `0x201` | `Data[6] bit6` | `0` | `1` |

## 補足

- `0x201 Data[7]` は常に `0x00`
- `share` は CAN に直接は載らず、`Normal -> Power -> Sensitive` のプロファイル切り替えに使う
- `options` は CAN に直接は載らず、`enable` のトグルに使う
- `enable == false` のとき、電流値として載る項目はすべて `255` に戻る

## 根拠

- [ARM9_ACV6_PACKET_AND_CAN.md](/Users/jinaoyagi/workspace/personal/ds4map/docs/ARM9_ACV6_PACKET_AND_CAN.md)
- [encoder.rs](/Users/jinaoyagi/workspace/personal/ds4map/src/output/formats/arm9/encoder.rs)
