/*
 * Copyright (C) 2022 - 2023 Partisia Blockchain Foundation
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 *
 */

import { ShardPutTransactionResponse } from "../client/ShardedClient";
import { Rpc, TransactionPayload } from "../client/TransactionData";

/**
 * Interface for a connected MPC wallet.
 */
export interface ConnectedWallet {
  readonly address: string;
  readonly signAndSendTransaction: (
    payload: TransactionPayload<Rpc>,
    cost?: string | number
  ) => Promise<ShardPutTransactionResponse>;
}
