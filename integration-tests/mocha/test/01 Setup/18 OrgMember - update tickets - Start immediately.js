const supertest = require('supertest');
const expect = require('chai').expect;
const mocha = require('mocha');
const tv4 = require('tv4');
const fs = require('fs');
const pm = require('../pm');const debug = require("debug");var log=debug('bn-api');

const baseUrl = supertest(pm.environment.get('server'));

const apiEndPoint = '/events/{{start_immediately_event_id}}/ticket_types/{{immediate_ticket_type_id}}';


var response;
var responseBody;


const patch = async function (request_body) {
    return baseUrl
        .patch(pm.substitute(apiEndPoint))
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




describe('OrgMember - update tickets - start immediately', function () {
    before(async function () {
        let requestBody = `{
	"start_date":"2017-11-21T00:00:00",
	"end_date": "8999-01-10T02:22:00",
	"visibility": "Always",
	"price_in_cents": 3000,
	"parent_id": null
}`;
        response = await patch(requestBody);
        expect(response.status).to.equal(200);
        requestBody = `{
	"start_date":null,
	"end_date": "8999-01-10T02:22:00",
	"visibility": "Always",
	"price_in_cents": 3000,
	"parent_id": null
}`;
        response = await patch(requestBody);
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


});

            
