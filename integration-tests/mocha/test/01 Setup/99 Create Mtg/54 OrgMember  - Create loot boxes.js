const supertest = require('supertest');
const expect = require('chai').expect;
const mocha = require('mocha');
const tv4 = require('tv4');
const fs = require('fs');
const pm = require('../../pm');const debug=require('debug');var log = debug('bn-api');

const baseUrl = supertest(pm.environment.get('server'));

const apiEndPoint = '/events/{{last_event_id}}/ticket_types';


var response;
var responseBody;


const post = async function (request_body) {
    return baseUrl
        .post(pm.substitute(apiEndPoint))
        .set('Accept', 'application/json')
        .set('Content-Type', 'application/json')
        .set('Authorization', pm.substitute('Bearer {{org_member_token}}'))

        .send(pm.substitute(request_body));
};

const get = async function (request_body) {
    return baseUrl
        .get(pm.substitute(apiEndPoint))

        .set('Authorization', pm.substitute('Bearer {{org_member_token}}'))

        .set('Accept', 'application/json')
        .send();
};

let requestBody = `{
    "name": "Collector Booster (3x Mythic, 5x Rare/Mythic)",
    "promo_image_url": "https://www.sadrobot.co.za/wp-content/uploads/2019/12/MTGTHB_EN_ClctrBstr_01_01-510x934.png",
    "capacity": 3,
    "ticket_type_type": "LootBox",
    	"start_date":"1982-02-01T02:22:00",
	"end_date": "9999-01-10T02:22:00",
	"price_in_cents": 1700,
	"limit_per_person": 0,
	"visibility": "Always",
	"rank":1,
    "contents": [
     
         {
           "event_id": "{{last_event_id}}",
           "min_rarity_id": "{{mythic}}",
           "max_rarity_id": "{{mythic}}",
           "quantity_per_box": 4
        },
         {
           "event_id": "{{last_event_id}}",
           "min_rarity_id": "{{uncommon}}",
           "max_rarity_id": "{{uncommon}}",
           "quantity_per_box": 6
        }
    ]
}`;


describe('OrgMember  - Create loot boxes', function () {
    before(async function () {
        this.timeout(100000);
        console.log(pm.substitute(requestBody));
        response = await post(requestBody);
        log(response.request.header);
        log(response.request.url);
        log(response.request._data);
        log(response.request.method);
        responseBody = JSON.stringify(response.body);
        //log(pm);
        log(response.status);
        log(responseBody);
    });

    after(async function () {
        // add after methods


        pm.environment.set("last_loot_box_id", JSON.parse(responseBody).id);
    });

    it("should be 201", function () {
        expect(response.status).to.equal(201);
    })


});


