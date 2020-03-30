const supertest = require('supertest');
const expect = require('chai').expect;
const mocha = require('mocha');
const tv4 = require('tv4');
const fs = require('fs');
const pm = require('../../pm');const debug=require('debug');var log = debug('bn-api');

const baseUrl = supertest(pm.environment.get('server'));

const apiEndPoint = '/events';


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
    "name": "Throne of Eldraine",
    "organization_id": "{{last_org_id}}",
    "venue_id": "{{last_venue_id}}",
    "event_start": "2020-11-13T12:00:00",
    "event_end": "2020-11-14T12:00:00",
    "promo_image_url": "https://gamepedia.cursecdn.com/mtgsalvation_gamepedia/6/64/ELD_booster.png?version=2a39525183d6458c78e1d44438a49d85",
    "event_type": "Music",
    "age_limit": "21"
}`;


describe('OrgMember  - Create Mtg Set - Throne of Eldraine', function () {
    before(async function () {
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


        pm.environment.set("last_mtg_event_id", JSON.parse(responseBody).id);
    });

    it("should be 201", function () {
        expect(response.status).to.equal(201);
    })


});


