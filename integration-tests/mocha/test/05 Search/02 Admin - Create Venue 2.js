const supertest = require('supertest');
const expect = require('chai').expect;
const mocha = require('mocha');
const tv4 = require('tv4');
const fs = require('fs');
const pm = require('../pm');const debug = require("debug");var log=debug('bn-api');

const baseUrl = supertest(pm.environment.get('server'));

const apiEndPoint = '/venues';


var response;
var responseBody;


const post = async function (request_body) {
    return baseUrl
        .post(pm.substitute(apiEndPoint))
        .set('Accept', 'application/json')
        .set('Content-Type', 'application/json')
        .set('Authorization', pm.substitute('Bearer {{token}}'))

        .send(pm.substitute(request_body));
};

const get = async function (request_body) {
    return baseUrl
        .get(pm.substitute(apiEndPoint))

        .set('Authorization', pm.substitute('Bearer {{token}}'))

        .set('Accept', 'application/json')
        .send();
};

let requestBody = `{
	"name":"Test venue_{{$timestamp}}",
	"address": "1 street street",
	"city": "City",
	"country": "Country",
	"phone": "5555555555",
	"google_place_id": null,
	"state": "California",
	"postal_code": "23233",
	"organization_ids": ["{{last_org_id}}"],
	"timezone": "America/Los_Angeles"
}`;


describe('Admin - Create Venue 2', function () {
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

        pm.environment.set("venue2_id", JSON.parse(responseBody).id);


    });

    it("should be 201", function () {
        expect(response.status).to.equal(201);
    })


});
