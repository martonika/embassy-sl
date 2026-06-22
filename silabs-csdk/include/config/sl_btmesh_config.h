/***************************************************************************//**
 * @file sl_btmesh_config.h
 * @brief Bluetooth Mesh stack config (btmesh_soc_empty parity)
 *
 * Overrides SDK default SL_BTMESH_CONFIG_MAX_PROV_BEARERS (2) with 3 so
 * PB-ADV + PB-GATT + proxy provisioning bearers fit the generated DCD.
 ******************************************************************************/
#ifndef SL_BTMESH_CONFIG_H
#define SL_BTMESH_CONFIG_H

#define SL_BTMESH_CONFIG_MAX_APP_BINDS       (4)
#define SL_BTMESH_CONFIG_MAX_SUBSCRIPTIONS       (4)
#define SL_BTMESH_CONFIG_MAX_NETKEYS       (4)
#define SL_BTMESH_CONFIG_MAX_APPKEYS       (4)
#define SL_BTMESH_CONFIG_NET_CACHE_SIZE       (16)
#define SL_BTMESH_CONFIG_RPL_SIZE       (32)
#define SL_BTMESH_CONFIG_MAX_SEND_SEGS       (4)
#define SL_BTMESH_CONFIG_MAX_RECV_SEGS       (4)
#define SL_BTMESH_CONFIG_MAX_VAS       (4)
#define SL_BTMESH_CONFIG_MAX_PROV_SESSIONS       (2)
#define SL_BTMESH_CONFIG_MAX_PROV_BEARERS       (3)
#define SL_BTMESH_CONFIG_GATT_TXQ_SIZE       (4)
#define SL_BTMESH_CONFIG_MAX_PROVISIONED_DEVICES       (0)
#define SL_BTMESH_CONFIG_MAX_PROVISIONED_DEVICE_APPKEYS       (0)
#define SL_BTMESH_CONFIG_MAX_PROVISIONED_DEVICE_NETKEYS       (0)
#define SL_BTMESH_CONFIG_MAX_FOUNDATION_CLIENT_CMDS       (0)
#define SL_BTMESH_CONFIG_MAX_FRIENDSHIPS       (1)
#define SL_BTMESH_CONFIG_FRIEND_MAX_SUBS_LIST       (5)
#define SL_BTMESH_CONFIG_FRIEND_MAX_TOTAL_CACHE       (32)
#define SL_BTMESH_CONFIG_FRIEND_MAX_SINGLE_CACHE      (32)
#define SL_BTMESH_CONFIG_APP_TXQ_SIZE       (5)
#define SL_BTMESH_CONFIG_SEQNUM_WRITE_INTERVAL_EXP       (16)
#define SL_BTMESH_CONFIG_ITS_KEY_CACHE_SIZE       (4)
#define SL_BTMESH_CONFIG_MAX_PROXY_ACCESS_CONTROL_LIST_ENTRIES  (8)
#define SL_BTMESH_CONFIG_LIMIT_PROV_CONCURRENT_KR   (16)

#endif
