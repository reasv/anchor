import { Layout } from "buffer-layout";
import { Idl } from "../idl";
import { IdlCoder } from "./idl";
import { sha256 } from "js-sha256";
import * as base64 from "base64-js";

/**
 * Number of bytes of the account discriminator.
 */
export const ACCOUNT_DISCRIMINATOR_SIZE = 8;

/**
 * Encodes and decodes account objects.
 */
export class AccountsCoder {
  /**
   * Maps account type identifier to a layout.
   */
  private accountLayouts: Map<string, Layout>;
  /**
   * Maps account discriminator to type identifier.
   */
  private accountNames: Map<string, string>;

  public constructor(idl: Idl) {
    if (idl.accounts === undefined) {
      this.accountLayouts = new Map();
      return;
    }
    const layouts: [string, Layout][] = idl.accounts.map((acc) => {
      return [acc.name, IdlCoder.typeDefLayout(acc, idl.types)];
    });

    this.accountLayouts = new Map(layouts);

    const accountNamesArray: [string, string][] = idl.accounts.map((accountType) => {
      return [base64.fromByteArray(accountDiscriminatorSync(accountType.name)), accountType.name] as [string, string]
    })
    this.accountNames = new Map(accountNamesArray)
  }
  /**
   * Get account type identifier from discriminator
   */
  public getAccountName(discriminator: Buffer): string {
    return this.accountNames.get(base64.fromByteArray(discriminator))
  }

  public async encode<T = any>(
    accountName: string,
    account: T
  ): Promise<Buffer> {
    const buffer = Buffer.alloc(1000); // TODO: use a tighter buffer.
    const layout = this.accountLayouts.get(accountName);
    const len = layout.encode(account, buffer);
    let accountData = buffer.slice(0, len);
    let discriminator = await accountDiscriminator(accountName);
    return Buffer.concat([discriminator, accountData]);
  }

  public decode<T = any>(accountName: string, ix: Buffer): T {
    // Chop off the discriminator before decoding.
    const data = ix.slice(8);
    const layout = this.accountLayouts.get(accountName);
    return layout.decode(data);
  }
}
// Synchronous version of accountDiscriminator
export function accountDiscriminatorSync(name: string): Buffer {
  return Buffer.from(sha256.digest(`account:${name}`)).slice(0, 8);
}
// Calculates unique 8 byte discriminator prepended to all anchor accounts.
export async function accountDiscriminator(name: string): Promise<Buffer> {
  return Buffer.from(sha256.digest(`account:${name}`)).slice(0, 8);
}
