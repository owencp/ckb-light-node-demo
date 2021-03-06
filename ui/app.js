function render_index() {
    fetch('templates/index.mustache')
    .then(response => response.text())
    .then(template => {
        request('accounts')
        .then(json => {
            const data = {
                'data': json.result.map(a => ({
                    address: a.address,
                    balance: (parseInt(a.balance) / 100000000).toFixed(8),
                    indexed_block_number: parseInt(a.indexed_block_number)
                 }))
            };
            const rendered = Mustache.render(template, data);
            document.getElementById('main').innerHTML = rendered;
        });
    });
}

function render_send() {
    fetch('templates/send.mustache')
    .then(response => response.text())
    .then(template => {
        request('accounts')
        .then(json => {
            const data = {
                'data': json.result.map(a => ({
                    address: a.address,
                    balance: (parseInt(a.balance) / 100000000).toFixed(8),
                 }))
            };
            const rendered = Mustache.render(template, data);
            document.getElementById('main').innerHTML = rendered;
        });
    });
}

function render_transactions() {
    fetch('templates/transactions.mustache')
    .then(response => response.text())
    .then(template => {
        request('account_transactions')
        .then(json => {
            const data = {
                'data': json.result
                .sort((a, b) => {
                    if (a.block_number > b.block_number) {
                        return -1;
                    }
                    if (a.block_number < b.block_number) {
                        return 1;
                    }
                    if (a.address > b.address) {
                        return -1;
                    }
                    if (a.address < b.address) {
                        return 1;
                    }
                    return 0;
                })
                .map(a => ({
                    address: a.address,
                    tx_hash: a.tx_hash,
                    balance_change: (a.balance_change / 100000000).toFixed(8),
                    block_number: parseInt(a.block_number)
                 }))
            };
            const rendered = Mustache.render(template, data);
            document.getElementById('main').innerHTML = rendered;
        });
    });
}

function do_transfer() {
    request('transfer',
        document.getElementById('from_address').selectedIndex,
        document.getElementById('to_address').value,
        '0x' + (document.getElementById('capacity').value * 100000000).toString(16))
    .then(json => {
        alert("success, tx hash: " + json.result)
    })
    return false;
}

function do_generate_account() {
    request('generate_account')
    .then(json => {
        window.location.href = "/";
    })
}

function request(method, ...params) {
    const rpc_uri = "http://localhost:8121";
    const req = {
        method: 'POST',
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify({
            jsonrpc: '2.0',
            id: guid(),
            method,
            params: Array.isArray(params) ? params : [params],
        }),
    };

    return fetch(rpc_uri, req)
        .then(res => res.json())
        .then(json => {
            if (json.error) {
                alert(json.error.message);
            } else {
                return json;
            }
        });
}

function guid() {
    return Math.random().toString(36).substring(2, 15) +
        Math.random().toString(36).substring(2, 15);
}