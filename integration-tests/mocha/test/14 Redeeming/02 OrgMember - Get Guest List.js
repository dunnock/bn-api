const supertest = require('supertest');
const expect = require('chai').expect;
const mocha = require('mocha');
const tv4 = require('tv4');
const fs = require('fs');
const pm = require('../pm');const debug = require("debug");var log=debug('bn-api');

const baseUrl = supertest(pm.environment.get('server'));

const apiEndPoint = '/events/{{last_event_id}}/guests?query=';

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

let requestBody = ``;

describe('OrgMember - Get Guest List', function () {
    before(async function () {
        response = await get(requestBody);
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
    });

    it("should be 200", function () {
        expect(response.status).to.equal(200);
    })
    ;

    it("guests should be present", function () {

        let json = JSON.parse(responseBody);
        expect(json.data.length).to.be.greaterThan(6);

        // Make sure last ticket instance is not set to be transferred
        for (i = 0; i < json.data.length; i++) {
          if (json.data[i].transfer_id == null) {
            pm.environment.set("last_ticket_instance_id", json.data[i].id);
            pm.environment.set("last_ticket_instance_redeem_key", json.data[i].redeem_key);
            break;
          }
        }
    });
});
