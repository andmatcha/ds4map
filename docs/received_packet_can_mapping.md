# 受信パケットと CAN ID / データ対応表

このドキュメントは、XBee 経由で `USART1` に DMA 受信した 31 byte のパケットを、どの `CAN ID` と `CAN payload` に変換しているかをコードから整理したものです。

参照元:

- `PacketAC_v3` 定義: `src/main.c`
- モード判定: `src/main.c`
- CAN 送信処理: `src/main.c`
- CAN 設定: `src/main.c`, `ARM_communication_Uplink.ioc`

## 1. 全体フロー

1. `USART1` で 31 byte を DMA 受信する。
2. 受信データ先頭 2 byte が `"AC"` なら `PacketAC_v3` として解釈する。
3. `flags` の bit4-5 をモード値として取り出す。
4. モードに応じて 2 個または 3 個の標準 CAN フレームへ分割して送信する。

## 2. 受信パケット `PacketAC_v3` の構造

`PacketAC_v3` は `__attribute__((packed))` 付きで定義されており、サイズは 31 byte です。

| Byte offset | Size | 型 | フィールド | 内容 |
| --- | ---: | --- | --- | --- |
| 0 | 1 | `char` | `header[0]` | `'A'` |
| 1 | 1 | `char` | `header[1]` | `'C'` |
| 2 | 1 | `uint8_t` | `seq` | シーケンス番号 |
| 3 | 1 | `uint8_t` | `flags` | モードなどの制御情報 |
| 4-17 | 14 | `uint16_t[7]` | `current` | 電流値 7 個 |
| 18-23 | 6 | `uint16_t[3]` | `angle` | 角度 3 個 |
| 24-29 | 6 | `int16_t[3]` | `vel` | 速度 3 個 |
| 30 | 1 | `uint8_t` | `extra_flags` | 追加フラグ |

`uint16_t` / `int16_t` は STM32 上ではリトルエンディアンで扱われるため、CAN データにも下位 byte, 上位 byte の順で格納されています。

## 3. モード判定

モード値は `flags` の bit4-5 から取り出されています。

```c
uint8_t mode = (packet->flags >> 4) & 0x03;
```

対応は以下です。

| `mode` | 意味 | 送信される CAN ID |
| ---: | --- | --- |
| `0` | IK Mode | `0x401`, `0x402`, `0x403` |
| `1` | Manual Mode | `0x200`, `0x201` |
| `2` | 未実装 | 送信なし |
| `3` | 未実装 | 送信なし |

## 4. 受信パケット内の `current[]` の意味

コード中のコメントと詰め替え方から、`current[]` の意味は以下のように読めます。

| `current` index | 意味 |
| ---: | --- |
| `current[0]` | `base_horizen_current` |
| `current[1]` | `base_roll_current` |
| `current[2]` | `joint1_current` |
| `current[3]` | `joint2_current` |
| `current[4]` | `joint3_current` |
| `current[5]` | `joint4_current` |
| `current[6]` | `gripper_current` |

`angle[]` と `vel[]` は以下です。

| 配列 | index | 意味 |
| --- | ---: | --- |
| `angle` | `0` | `joint1_angle` |
| `angle` | `1` | `joint2_angle` |
| `angle` | `2` | `joint3_angle` |
| `vel` | `0` | `joint1_vel` |
| `vel` | `1` | `joint2_vel` |
| `vel` | `2` | `joint3_vel` |

## 5. IK Mode (`mode == 0`) の CAN 対応

### CAN ID `0x401` (`IK_UPLINK_CANID_1`)

角度 3 個を送ります。

| CAN byte | 内容 |
| ---: | --- |
| 0 | `angle[0]` LSB |
| 1 | `angle[0]` MSB |
| 2 | `angle[1]` LSB |
| 3 | `angle[1]` MSB |
| 4 | `angle[2]` LSB |
| 5 | `angle[2]` MSB |
| 6 | `0x00` |
| 7 | `0x00` |

意味:

- `joint1_angle`
- `joint2_angle`
- `joint3_angle`

### CAN ID `0x402` (`IK_UPLINK_CANID_2`)

速度 3 個と `extra_flags` を送ります。

| CAN byte | 内容 |
| ---: | --- |
| 0 | `vel[0]` LSB |
| 1 | `vel[0]` MSB |
| 2 | `vel[1]` LSB |
| 3 | `vel[1]` MSB |
| 4 | `vel[2]` LSB |
| 5 | `vel[2]` MSB |
| 6 | `extra_flags` |
| 7 | `0x00` |

意味:

- `joint1_vel`
- `joint2_vel`
- `joint3_vel`
- `extra_flags`

### CAN ID `0x403` (`IK_UPLINK_CANID_3`)

ベース 2 軸の電流と、`joint4` / `gripper` の電流を送ります。

| CAN byte | 内容 |
| ---: | --- |
| 0 | `current[0]` LSB |
| 1 | `current[0]` MSB |
| 2 | `current[1]` LSB |
| 3 | `current[1]` MSB |
| 4 | `current[5]` LSB |
| 5 | `current[5]` MSB |
| 6 | `current[6]` LSB |
| 7 | `current[6]` MSB |

意味:

- `base_horizen_current`
- `base_roll_current`
- `joint4_current`
- `gripper_current`

注意:

- `current[2]` `current[3]` `current[4]`、つまり `joint1_current` `joint2_current` `joint3_current` は IK Mode では CAN に送られていません。

## 6. Manual Mode (`mode == 1`) の CAN 対応

### CAN ID `0x200` (`MANUAL_UPLINK_CANID_1`)

前半 4 個の電流を送ります。

| CAN byte | 内容 |
| ---: | --- |
| 0 | `current[0]` LSB |
| 1 | `current[0]` MSB |
| 2 | `current[1]` LSB |
| 3 | `current[1]` MSB |
| 4 | `current[2]` LSB |
| 5 | `current[2]` MSB |
| 6 | `current[3]` LSB |
| 7 | `current[3]` MSB |

意味:

- `base_horizen_current`
- `base_roll_current`
- `joint1_current`
- `joint2_current`

### CAN ID `0x201` (`MANUAL_UPLINK_CANID_2`)

後半 3 個の電流と `extra_flags` を送ります。

| CAN byte | 内容 |
| ---: | --- |
| 0 | `current[4]` LSB |
| 1 | `current[4]` MSB |
| 2 | `current[5]` LSB |
| 3 | `current[5]` MSB |
| 4 | `current[6]` LSB |
| 5 | `current[6]` MSB |
| 6 | `extra_flags` |
| 7 | `0x00` |

意味:

- `joint3_current`
- `joint4_current`
- `gripper_current`
- `extra_flags`

## 7. 一覧表

| モード | CAN ID | Data[0:7] の意味 |
| --- | --- | --- |
| IK | `0x401` | `joint1_angle`, `joint2_angle`, `joint3_angle`, `0`, `0` |
| IK | `0x402` | `joint1_vel`, `joint2_vel`, `joint3_vel`, `extra_flags`, `0` |
| IK | `0x403` | `base_horizen_current`, `base_roll_current`, `joint4_current`, `gripper_current` |
| Manual | `0x200` | `base_horizen_current`, `base_roll_current`, `joint1_current`, `joint2_current` |
| Manual | `0x201` | `joint3_current`, `joint4_current`, `gripper_current`, `extra_flags`, `0` |

## 8. 通信設定として読み取れる内容

- UART 受信ポート: `USART1`
- UART ボーレート: `57600`
- 受信サイズ: `31 byte`
- CAN フレーム種別: 標準 ID (`CAN_ID_STD`)
- CAN DLC: 常に `8`
- CAN bitrate: `.ioc` 上では `1000000` bps

## 9. 実装上の注意点

### `startCANTransmit()` の呼び出し引数

メインループでは `checkXbeeData(rxBuffer, &myPacket)` 成功後に次を呼んでいます。

```c
startCANTransmit(rxBuffer);
```

ただし関数宣言は以下です。

```c
void startCANTransmit(PacketAC_v3* packet);
```

`rxBuffer` は型としては `uint8_t *` なので、意図としては `startCANTransmit(&myPacket);` の可能性が高いです。  
今回の対応表は、`startCANTransmit()` 内部のフィールド参照ロジックそのものを根拠に整理しています。

### DMA の扱い

コメントでは circular モード受信を意図していますが、31 byte 固定長の 1 パケット受信完了を前提に `HAL_UART_RxCpltCallback()` でフラグを立てる実装です。  
したがって、この文書は「31 byte が 1 回きれいに受信完了したとき」のマッピング仕様として読むのが安全です。

