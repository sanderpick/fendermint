import itertools as itt
import requests

sql = "countries.sql"
batch_size = 10
start_sequence = 0
# host = "http://34.125.163.42:26650"
host = "http://127.0.0.1:26650"

with open(sql) as file:
    lines = file.readlines()
    batches = [[e[1] for e in grp] for k, grp in itt.groupby(enumerate(lines), key=lambda x: x[0] // batch_size % 2)]

    for i, b in enumerate(batches):
        b = [s.rstrip() for s in b]
        stmts = ''.join(map(str, itt.chain.from_iterable(b)))
        x = requests.post(host + "/v1/execute", json={'stmts': stmts,
                                                      'sequence': start_sequence + i,
                                                      'gas_limit': 10000000000000})
        print(x.json())
