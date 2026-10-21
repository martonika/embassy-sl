/**
 * Force-link strong RAIL BLE PHY pointers for BRD4186C (39 MHz HFXO).
 *
 * librail_efr32xg24_*.a provides weak NULL stubs for RAIL_BLE_Phy*Mbps*.
 * Archive link order can leave those NULLs in the image so
 * sl_rail_ble_config_phy_1_mbps never loads a channel config →
 * blePhy stays UNDEFINED → config_channel returns INVALID_STATE (0x2) →
 * start_tx returns INVALID_CALL (0xE).
 *
 * Linked via rust-lld `-usilabs_force_rail_ble_phys` so this .o is always
 * pulled and the strong PHY pointers override the weak NULL stubs.
 */
#include "rail_types.h"
#include "sl_rail_ble_config_39MHz.h"

const RAIL_ChannelConfig_t *const RAIL_BLE_Phy1MbpsViterbi =
  &sl_rail_ble_phy_1Mbps_viterbi_39MHz_channelConfig;

const RAIL_ChannelConfig_t *const RAIL_BLE_Phy2MbpsViterbi =
  &sl_rail_ble_phy_2Mbps_viterbi_39MHz_channelConfig;

const RAIL_ChannelConfig_t *const RAIL_BLE_Phy125kbps =
  &sl_rail_ble_phy_125kbps_39MHz_channelConfig;

const RAIL_ChannelConfig_t *const RAIL_BLE_Phy500kbps =
  &sl_rail_ble_phy_500kbps_39MHz_channelConfig;

const RAIL_ChannelConfig_t *const RAIL_BLE_PhySimulscan =
  &sl_rail_ble_phy_simulscan_39MHz_channelConfig;

const RAIL_ChannelConfig_t *const RAIL_BLE_Phy2MbpsAox =
  &sl_rail_ble_phy_2Mbps_aox_39MHz_channelConfig;

void silabs_force_rail_ble_phys(void) {}
