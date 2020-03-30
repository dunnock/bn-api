const supertest = require('supertest');
const expect = require('chai').expect;
const mocha = require('mocha');
const tv4 = require('tv4');
const fs = require('fs');
const pm = require('../../pm');const debug = require("debug");var log=debug('bn-api');
const mtg = require('mtgsdk');

const baseUrl = supertest(pm.environment.get('server'));

const apiEndPoint = '/events/{{last_mtg_event_id}}/ticket_types';


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


let rarities = {
    "Mythic": {varName: "mythic", qty: 10},
    "Rare": {varName: "rare", qty: 100},
    "Uncommon": {varName: "uncommon", qty: 300 },
    "Common": {varName: "common", qty: 1000},
};

var responses = [];

describe('OrgMember - create cards', function () {
    before(async function () {
        this.timeout(100000);
        responses = await mtg.card.where({ set: "GRN" })
            .then(async cards => {

            return await cards.map(async card => {
                console.log(card);
                if (card.imageUrl && rarities[card.rarity]) {


                    let requestBody = `{
	"name":"${card.name} (${card.rarity})",
	"capacity": ${rarities[card.rarity].qty},
	"rarity_id": "{{${rarities[card.rarity].varName}}}",
	"promo_image_url": "${card.imageUrl}",
	"start_date":"1982-02-01T02:22:00",
	"end_date": "9999-01-10T02:22:00",
	"price_in_cents": 2500,
	"limit_per_person": 0,
	"visibility": "Hidden",
	"rank":1,
	"ticket_pricing":[]
}`;
                    console.log(requestBody);
                    response = await post(requestBody);
                    console.log(response.status);
                    return response;
                }
                return null;
        });}).catch(err => console.log(err));


        // response = await post(requestBody);
        // log(response.request.header);
        // log(response.request.url);
        // log(response.request._data);
        // log(response.request.method);
        // responseBody = JSON.stringify(response.body);
        // //log(pm);
        // log(response.status);
        // log(responseBody);
        responses = await Promise.all(responses);
    });

    after(async function () {
        // add after methods

        // pm.environment.set("last_ticket_type_id", JSON.parse(responseBody).id);
        //
        // pm.environment.set("default_ticket_type_id", JSON.parse(responseBody).id);


    });
    //
    it("should be 201", function () {
        console.log(responses);
        for (var i=0;i<responses.length;i++){
            if (responses[i])
            {
        expect(responses[i].status).to.equal(201);}}
    })


});


