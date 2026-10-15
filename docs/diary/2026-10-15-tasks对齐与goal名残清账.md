# 2026-10-15 tasks 对齐批与 show 行名残清账

## 事

evo 台批159续单转派:以本仓 hst 2.15.0 实态核对 evo tasks skill 三动词面(view/set/to),命令形加旗标加输出列加唤醒语义四面对照毕,漂移 9 键详单投 evo 台(D4/D7 skill 事实漂移彼台自修,D1 至 D3 加 D5 加 D6 加 D9 代码面转呈总台裁定,D8 三注记随单)。本仓自留一笔即此批:clap show 行文案「无 task 给 goal.present=false」名残(goal 键时代残语,实出 kv 键为 task.present,REQ-054 别名摘除清账漏网),改 task.present。doctor 的 project goal present 措辞与 REQ-045 历史登记属存量语义与历史时点事实,比照 2067529 doctor 豁免先例不在清列。

## 证

门禁:双版 fmt 绿(仓 fmt 绿;dev 静检 hook edition 2024 口径下 main.rs 两长行包行并绿,1158 加 1869,系仓 edition 2021 与静检 2024 存量分叉,他文件同分叉约 40 处不在本批)加 clippy 零新告警(存量 warning 集 in doctor 加 agents 加 diagnose,均未触碰)加测试 244 加 63 绿加 aidoc 26 件 check 干净加 mdcharlint 与 rumdl 与 ref-scan 绿。存量红两处实证均 HEAD 2067529 同形非本批引入:PEVO PASS 10 加 FAIL 2(PE-10 与 PE-11 落 docs/aidoc 生成面与九月老 diary);heading-scan rc=1(v2.14 收口 diary 第十三笔标题含括号)。批159 核心实证两条:hst goal show 实弹 unrecognized subcommand(2.15.0 别名摘除生效,evo 台 D1);hst task show kv 首行恒 task.present 探针实证,文本在第二行,故 evo tasks view 直读取首行文本永不出面(evo 台 D6)。

## 次

evo 台候一笔提交回报即闭环;PEVO checker 口径漂移记档候核;edition 双版 fmt 分叉余面约 40 处候逐文件并绿。
