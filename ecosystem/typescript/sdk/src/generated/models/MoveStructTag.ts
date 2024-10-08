/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

/**
 * String representation of a MoveStructTag (on-chain Move struct type). This exists so you
 * can specify MoveStructTags as path / query parameters, e.g. for get_events_by_event_handle.
 *
 * It is a combination of:
 * 1. `move_module_address`, `module_name` and `struct_name`, all joined by `::`
 * 2. `struct generic type parameters` joined by `, `
 *
 * Examples:
 * * `0x1::coin::CoinStore<0x1::diem_coin::DiemCoin>`
 * * `0x1::account::Account`
 *
 * Note:
 * 1. Empty chars should be ignored when comparing 2 struct tag ids.
 * 2. When used in an URL path, should be encoded by url-encoding (AKA percent-encoding).
 *
 * See [doc](https://diem.dev/concepts/accounts) for more details.
 *
 */
export type MoveStructTag = string;
