# multi_pair_backtest_rs
支持多种交易对的量化交易回测框架
## 目录架构
```
- doc // 文档
- src
    - bin // 可执行文件入口
    - strategy // 策略
        - strategy_order // 策略订单 及 订单管理器
        - strategy_trading_pair // 策略交易对 及 交易对管理器
    - runner // 执行器
        - strategy_runner // 回测执行器
            - runner_order // 执行器订单 及 订单管理器
            - runner_trading_pair // 执行器交易对 及 交易对管理器
            - runner_data_manager // 执行器数据管理器
        - okx_runner // 实盘执行器 对接okx
        - binance_runner // 实盘执行器 对接bn
    - order
        - order_enum // 订单公共属性枚举
    - data // 数据管理
        - kline // k线
        - funding_rate // 资金费率
    - assert // 资产管理
        - trading_pair // 交易对
        - assert // 资产 及 资产管理器
```