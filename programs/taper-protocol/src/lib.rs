use anchor_lang::prelude::*;
use anchor_spl::token_2022_extensions::transfer_fee::{
    harvest_withheld_tokens_to_mint, withdraw_withheld_tokens_from_mint,
    HarvestWithheldTokensToMint, WithdrawWithheldTokensFromMint,
};
use anchor_spl::token_interface::{
    burn, transfer_checked, Burn, Mint, TokenAccount, TokenInterface, TransferChecked,
};

declare_id!("EuR5xFdq52NS8fqACtNxnzAPQDiy4XKEjcFmP8rsedMf");

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    // Required fields
    name: "Taper Protocol",
    project_url: "https://taperprotocol.net",
    contacts: "email:taperprotocol@gmail.com,twitter:@taperprotocol,link:https://t.me/taperprotocol",
    policy: "https://github.com/Taper-protocol/core/blob/main/SECURITY.md",

    // Optional fields
    preferred_languages: "en,ru",
    source_code: "https://github.com/Taper-protocol/core",
    auditors: "None"
}

// Seed of the Vault authority PDA
pub const VAULT_AUTHORITY_SEED: &[u8] = b"vault-authority";

// Distribution shares (in percent) of the harvested fees:
// 80% burned (0.04% of transfer volume), the remaining 20% to the caller (0.01%).
const BURN_PERCENT: u128 = 80;
const PERCENT_DENOMINATOR: u128 = 100;

/// Splits `total` into (burn, caller).
/// Intermediate math is done in u128, so no overflow is possible for any u64 value.
/// The rounding remainder goes to the caller, so the two shares always sum to `total`.
pub fn split_amount(total: u64) -> Result<(u64, u64)> {
    let burn_amount = u64::try_from((total as u128) * BURN_PERCENT / PERCENT_DENOMINATOR)
        .map_err(|_| error!(ErrorCode::MathOverflow))?;

    let caller_amount = total
        .checked_sub(burn_amount)
        .ok_or(ErrorCode::MathOverflow)?;

    Ok((burn_amount, caller_amount))
}

#[program]
pub mod taper_protocol {
    use super::*;

    pub fn harvest_and_distribute<'info>(
        ctx: Context<'info, HarvestAndDistribute<'info>>,
    ) -> Result<()> {
        require!(
            !ctx.remaining_accounts.is_empty(),
            ErrorCode::NoAccountsToHarvest
        );

        // In Anchor 1.x CpiContext takes the program id (Pubkey), not an AccountInfo
        let token_program_key = ctx.accounts.token_program.key();

        // Signer seeds for the vault_authority PDA
        let bump = ctx.bumps.vault_authority;
        let signer_seeds: &[&[&[u8]]] = &[&[VAULT_AUTHORITY_SEED, &[bump]]];

        // 1. Harvest withheld fees from token accounts to the Mint (permissionless, no signature needed)
        harvest_withheld_tokens_to_mint(
            CpiContext::new(
                token_program_key,
                HarvestWithheldTokensToMint {
                    token_program_id: ctx.accounts.token_program.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                },
            ),
            ctx.remaining_accounts.to_vec(),
        )?;

        // 2. Withdraw withheld fees from the Mint to the Vault (PDA's ATA).
        // The vault_authority PDA signs: it must be the Mint's withdraw withheld authority.
        withdraw_withheld_tokens_from_mint(
            CpiContext::new(
                token_program_key,
                WithdrawWithheldTokensFromMint {
                    token_program_id: ctx.accounts.token_program.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                    destination: ctx.accounts.vault.to_account_info(),
                    authority: ctx.accounts.vault_authority.to_account_info(),
                },
            )
            .with_signer(signer_seeds),
        )?;

        // 3. Reload the account state and read the resulting balance
        ctx.accounts.vault.reload()?;
        let total = ctx.accounts.vault.amount;

        require!(total > 0, ErrorCode::NothingToDistribute);

        // 4. Compute the shares (80% Burn, 20% Caller Reward) in u128
        let (burn_amount, caller_amount) = split_amount(total)?;

        // 5. Burn 80% of the collected tokens
        burn(
            CpiContext::new(
                token_program_key,
                Burn {
                    mint: ctx.accounts.mint.to_account_info(),
                    from: ctx.accounts.vault.to_account_info(),
                    authority: ctx.accounts.vault_authority.to_account_info(),
                },
            )
            .with_signer(signer_seeds),
            burn_amount,
        )?;

        // 6. Pay the remaining 20% to the caller
        transfer_checked(
            CpiContext::new(
                token_program_key,
                TransferChecked {
                    from: ctx.accounts.vault.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.caller_token_account.to_account_info(),
                    authority: ctx.accounts.vault_authority.to_account_info(),
                },
            )
            .with_signer(signer_seeds),
            caller_amount,
            ctx.accounts.mint.decimals,
        )?;

        // 7. Emit the event
        emit!(DistributionEvent {
            total_collected: total,
            burned: burn_amount,
            caller_reward: caller_amount,
            caller: ctx.accounts.caller.key(),
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct HarvestAndDistribute<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,

    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,

    /// Vault is the canonical associated token account (ATA) of the `vault_authority` PDA
    /// for this Mint and this token program. No other token account owned by the PDA
    /// can be substituted: the address is verified by derivation.
    /// The ATA must be created in advance (once, by a client).
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = vault_authority,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        seeds = [VAULT_AUTHORITY_SEED],
        bump
    )]
    /// CHECK: PDA authority, validated by seeds and bump
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        token::mint = mint,
        token::authority = caller.key(),
    )]
    pub caller_token_account: InterfaceAccount<'info, TokenAccount>,

    /// Only the Token-2022 program is accepted: the Transfer Fee extension
    /// (harvest / withdraw withheld tokens) does not exist in the legacy SPL Token program.
    #[account(address = anchor_spl::token_2022::ID)]
    pub token_program: Interface<'info, TokenInterface>,
}

#[event]
pub struct DistributionEvent {
    pub total_collected: u64,
    pub burned: u64,
    pub caller_reward: u64,
    pub caller: Pubkey,
}

#[error_code]
pub enum ErrorCode {
    #[msg("No accounts were provided to harvest fees from.")]
    NoAccountsToHarvest,
    #[msg("The Vault has no tokens available for distribution.")]
    NothingToDistribute,
    #[msg("Math overflow while computing shares.")]
    MathOverflow,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_sums_to_total() {
        for total in [1u64, 9, 10, 99, 100, 101, 1_000_003, u64::MAX / 2, u64::MAX] {
            let (b, c) = split_amount(total).unwrap();
            assert_eq!(b as u128 + c as u128, total as u128);
        }
    }

    #[test]
    fn split_exact_values() {
        assert_eq!(split_amount(100).unwrap(), (80, 20));
        assert_eq!(split_amount(1).unwrap(), (0, 1));
        assert_eq!(split_amount(10).unwrap(), (8, 2));
    }

    #[test]
    fn split_does_not_overflow_on_max() {
        // In u64 `u64::MAX * 80` would overflow; in u128 it does not.
        let (b, c) = split_amount(u64::MAX).unwrap();
        assert!(b > c && c > 0);
    }
}