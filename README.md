

**1. Deploy the contract**

```bash
cd contracts
cargo stylus deploy --endpoint='http://localhost:8547' --private-key="0xb6b15c8cb491557369f3c7d2c287b053eb229daa9c22138887752191c9520659"
```

**2. Create environment configuration**

Create a `.env` file in the `game/` directory with the following content:

```bash
# Blockchain Configuration
RPC_URL=http://localhost:8547
STYLUS_CONTRACT_ADDRESS=<deployed_contract_address>
PRIVATE_KEY=0xb6b15c8cb491557369f3c7d2c287b053eb229daa9c22138887752191c9520659
```

Replace `<deployed_contract_address>` with the actual contract address from step 1.

**3. Run the game**

```bash
cd game
cargo run
```

**Note:** The blockchain functionality will only work if the `.env` file is properly configured. If the environment variables are missing or incorrect, the game will run without blockchain features.