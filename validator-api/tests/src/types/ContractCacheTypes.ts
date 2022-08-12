export type AllMixnodes = {
  pledge_amount: PledgeAmount;
  total_delegation: TotalDelegation;
  owner: string;
  layer: string;
  block_height: number;
  mix_node: Mixnode;
  proxy: string;
  accumulated_rewards: string;
};

export type PledgeAmount = {
  denom: string;
  amount: string;
};

export type TotalDelegation = {
  denom: string;
  amount: string;
};

export type Mixnode = {
  host: string;
  mix_port: number;
  verloc_port: number;
  http_api_port: number;
  sphinx_key: string;
  identity_key: string;
  version: string;
  profit_margin_percent: number;
};

export type MixnodesDetailed = {
  mixnode_bond: AllMixnodes;
  stake_saturation: number;
  uptime: number;
  estimated_operator_apy: number;
  estimated_delegators_apy: number;
};

export type AllGateways = {
  pledge_amount: PledgeAmount;
  owner: string;
  block_height: number;
  gateway: Gateway;
  proxy: string;
};

export type Gateway = {
  host: string;
  mix_port: number;
  clients_port: number;
  location: string;
  sphinx_key: string;
  identity_key: string;
  version: string;
};
