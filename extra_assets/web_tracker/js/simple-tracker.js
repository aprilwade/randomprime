'use strict';

class ItemCounterElement extends HTMLElement {
    connectedCallback() {
        this.count = 0;
    }

    get max() {
        return Number(this.getAttribute('max') || '1');
    }

    get increment() {
        return Number(this.getAttribute('increment') || '1');
    }

    get count() {
        return Number(this.getAttribute('count') || '0');
    }

    get item() {
        return String(this.getAttribute('item'));
    }

    set count(value) {
        let num = Number(value);
        if (num > this.max) {
            num = this.max;
        }
        if (num < 0) {
            num = 0;
        }
        this.setAttribute('count', String(num));
        let countSpan = this.querySelector('[counter]');
        if (countSpan) {
            countSpan.innerHTML = `${num}`;
        }
        let toggler = document.querySelector(`item-toggle[item=${this.item}]`);
        if (toggler) {
            toggler.setAttribute('count', String(num));
        }
    }

    incrementCount() {
        if (this.max === 1 && this.count === 1) {
            this.count = 0;
        } else {
            this.count += this.increment;
        }
    }

    decrementCount() {
        this.count -= this.increment;
    }
}

class ItemToggleElement extends HTMLElement {
    connectedCallback() {
        this.setAttribute('count', '0');
    }

    get item() {
        return String(this.getAttribute('item'));
    }
}

class NewTabLinkElement extends HTMLElement {
    connectedCallback() {
      this.addEventListener('click', event => {
        window.open(window.location.href, 'Prime Item Tracker', 'resizable=no, toolbar=no, scrollbars=no, menubar=no, status=no, directories=no, width=400, height=610');
      });
    }
}

customElements.define('item-counter', ItemCounterElement);
customElements.define('item-toggle', ItemToggleElement);
customElements.define('newtab-link', NewTabLinkElement);

var ITEM_TYPES = [
    ["missile-count", 4],
    ["etank-count", 24],
    ["pb-count", 7],

    ["missile", 4],
    ["super-missile", 11],
    ["wavebuster", 28],
    ["ice-spreader", 14],
    ["flamethrower", 8],

    ["morph", 16],
    ["bombs", 6],
    ["boost", 18],
    ["spider", 19],
    ["power-bombs", 7],

    ["charge", 10],
    ["wave", 2],
    ["ice", 1],
    ["plasma", 3],
    ["grapple", 12],

    ["truth", 29],
    ["strength", 30],
    ["elder", 31],
    ["wild", 32],
    ["lifegiver", 33],
    ["warrior", 34],
    ["chozo", 35],
    ["nature", 36],
    ["sun", 37],
    ["world", 38],
    ["spirit", 39],
    ["newborn", 40],

    ["space-jump", 15],
    ["thermal", 9],
    ["xray", 13],

    ["varia", 22],
    ["gravity", 21],
    ["phazon", 23],

    ["scan", 5],
];

function connectWebSocket() {
    var reconnectTimeout = 1000;
    var forceReconnectId;
    var ws = new WebSocket("ws://" + location.host + "/tracker");
    //var ws = new WebSocket("ws://" + "192.168.1.141:80" + "/tracker");

    ws.onmessage = function(event) {
        reconnectTimeout = 1000;

        // Reset the one-minute timer before forcing a reconnect
        clearTimeout(forceReconnectId);
        forceReconnectId = setTimeout(function() { ws.close() }, 60000);

        let data = JSON.parse(event.data);
        console.log(data);
        if(data.inventory != undefined) {

            ITEM_TYPES.forEach(function(item) {
                let elem = document.querySelector(`item-counter[item=${item[0]}]`);
                if(elem) {
                    elem.count = data.inventory[item[1]];
                }
            })

            let suits = document.querySelector('item-counter[item=suits]');
            if(suits) {
                suits.count = data.inventory[21]
                            + data.inventory[22]
                            + data.inventory[23];
            }

            let artifacts = document.querySelector('item-counter[item=artifacts]');
            if(artifacts) {
                artifacts.count = data.inventory[29] + data.inventory[30]
                                + data.inventory[31] + data.inventory[32]
                                + data.inventory[33] + data.inventory[34]
                                + data.inventory[35] + data.inventory[36]
                                + data.inventory[37] + data.inventory[38]
                                + data.inventory[39] + data.inventory[40];
            }
        }
    };

    ws.onopen = function(e) {
        console.log(e);
        forceReconnectId = setTimeout(function() { ws.close() }, 60000);
    }

    ws.onclose = function(e) {
        console.log(e);
        setTimeout(connectWebSocket, reconnectTimeout);
    }

    ws.onerror = function(e) {
        console.log(e);
        reconnectTimeout = Math.max(reconnectTimeout * 2, 30000);
        ws.close();
    };
}
connectWebSocket();
